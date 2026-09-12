import { test, expect, type Page, type WebSocketRoute } from "@playwright/test";

// Control the transport boundary to exercise disconnects and delayed authentication
// deterministically; the page, styles, DOM events and WebSocket helper are real.
async function transport(page: Page) {
  const sockets: WebSocketRoute[] = [];
  const sent: any[] = [];
  await page.routeWebSocket("**/ws", (ws) => {
    sockets.push(ws);
    ws.onMessage((data) => sent.push(JSON.parse(String(data))));
  });
  return { sockets, sent };
}
const snapshot = (overrides = {}) => ({
  type: "snapshot",
  accepting: false,
  roundId: 0,
  players: [{ id: "p1", name: "Alex", connected: true }],
  sequence: [],
  lanUrls: ["http://192.168.1.10:7423/"],
  you: { id: "p1", role: "player", place: null },
  ...overrides,
});
async function open(page: Page, path: string) {
  const t = await transport(page);
  await page.goto(path);
  await expect.poll(() => t.sockets.length).toBe(1);
  return t;
}

test("hidden result panel does not cover name entry", async ({ page }) => {
  await open(page, "/player.html");
  await expect(page.locator("#place")).toBeHidden();
  await page.locator("#name").fill("Alex");
  await expect(page.locator("#name")).toHaveValue("Alex");
});

test("player states follow snapshots and only buzz after fresh authentication", async ({
  page,
}) => {
  const t = await open(page, "/player.html");
  await page.locator("#name").fill("Alex");
  await page.locator("button[type=submit]").click();
  t.sockets[0].send(
    JSON.stringify({ type: "helloOk", role: "player", playerId: "p1" }),
  );
  t.sockets[0].send(JSON.stringify(snapshot()));
  await expect(page.locator("#join")).toBeHidden();
  await expect(page.locator("#buzzer")).toBeVisible();
  await expect(page.locator("#buzzer")).toBeDisabled();
  t.sockets[0].send(JSON.stringify(snapshot({ accepting: true, roundId: 1 })));
  await expect(page.locator("#buzzer")).toBeEnabled();
  await page
    .locator("#buzzer")
    .dispatchEvent("pointerdown", { button: 0, isPrimary: true });
  await expect
    .poll(() => t.sent.filter((m) => m.type === "buzz").length)
    .toBe(1);
  t.sockets[0].send(
    JSON.stringify(
      snapshot({
        accepting: true,
        roundId: 1,
        you: { role: "player", id: "p1", place: 1 },
      }),
    ),
  );
  await expect(page.locator("#buzzer")).toBeHidden();
  await expect(page.locator("#place")).toContainText("1st");
  t.sockets[0].send(
    JSON.stringify(
      snapshot({
        accepting: false,
        roundId: 1,
        you: { role: "player", id: "p1", place: 1 },
      }),
    ),
  );
  await expect(page.locator("#place")).toBeVisible();
  t.sockets[0].send(JSON.stringify(snapshot({ accepting: true, roundId: 2 })));
  await expect(page.locator("#place")).toBeHidden();
  await expect(page.locator("#buzzer")).toBeEnabled();
  t.sockets[0].close();
  await expect(page.locator("#buzzer")).toBeDisabled();
  await expect.poll(() => t.sockets.length).toBe(2);
  await expect(page.locator("#buzzer")).toBeDisabled();
  await page
    .locator("#buzzer")
    .dispatchEvent("pointerdown", { button: 0, isPrimary: true });
  t.sockets[1].send(JSON.stringify(snapshot({ accepting: true, roundId: 2 })));
  await expect(page.locator("#buzzer")).toBeDisabled();
  t.sockets[1].send(
    JSON.stringify({ type: "helloOk", role: "player", playerId: "p1" }),
  );
  await expect(page.locator("#buzzer")).toBeDisabled();
  t.sockets[1].send(JSON.stringify(snapshot({ accepting: true, roundId: 2 })));
  await expect(page.locator("#buzzer")).toBeEnabled();
  expect(t.sent.filter((m) => m.type === "buzz")).toHaveLength(1);
  await page.locator("#buzzer").focus();
  await page.keyboard.press("Space");
  await expect
    .poll(() => t.sent.filter((m) => m.type === "buzz").length)
    .toBe(2);
});

test("host controls stay disabled until authenticated snapshot and after disconnection", async ({
  page,
}) => {
  const t = await open(page, "/board.html?k=test");
  await expect(page.locator("#arm")).toBeDisabled();
  t.sockets[0].send(JSON.stringify({ type: "helloOk", role: "board" }));
  await expect(page.locator("#arm")).toBeDisabled();
  t.sockets[0].send(JSON.stringify(snapshot({ you: { role: "board" } })));
  await expect(page.locator("#arm")).toBeEnabled();
  await page.locator("#arm").focus();
  await page.keyboard.press("Enter");
  await expect
    .poll(() => t.sent.filter((m) => m.type === "arm").length)
    .toBe(1);
  t.sockets[0].close();
  await expect(page.locator("#arm")).toBeDisabled();
  await expect(page.locator("#reset")).toBeDisabled();
});

test("board preserves results when closed and join codes can be collapsed", async ({
  page,
}) => {
  const t = await open(page, "/board.html?k=test");
  t.sockets[0].send(JSON.stringify({ type: "helloOk", role: "board" }));
  const sequence = [
    { playerId: "p1", name: "Alex", place: 1 },
    { playerId: "p2", name: "Sam", place: 2 },
  ];
  t.sockets[0].send(
    JSON.stringify(
      snapshot({
        accepting: true,
        roundId: 1,
        sequence,
        you: { role: "board" },
      }),
    ),
  );
  await expect(page.locator("#main")).toHaveText("Alex");
  await expect(page.locator("#list li")).toHaveCount(2);
  await expect(page.locator("#qrs canvas")).toHaveCount(1);
  await page.getByRole("button", { name: "Hide join codes" }).click();
  await expect(page.locator("#qrs")).toBeHidden();
  await page.getByRole("button", { name: "Show join codes" }).click();
  await expect(page.locator("#qrs")).toBeVisible();
  t.sockets[0].send(
    JSON.stringify(snapshot({ roundId: 1, sequence, you: { role: "board" } })),
  );
  await expect(page.locator("#main")).toHaveText("Alex");
  t.sockets[0].send(
    JSON.stringify(
      snapshot({ accepting: true, roundId: 2, you: { role: "board" } }),
    ),
  );
  await expect(page.locator("#list li")).toHaveCount(0);
  await expect(page.locator("#main")).not.toHaveText("Alex");
});

test("join errors stay inline and allow retry; invalid host cannot act", async ({
  page,
}) => {
  const t = await open(page, "/player.html");
  const dialogs: string[] = [];
  page.on("dialog", async (dialog) => {
    dialogs.push(dialog.message());
    await dialog.dismiss();
  });
  await page.locator("#name").fill("Alex");
  await page.locator("button[type=submit]").click();
  t.sockets[0].send(
    JSON.stringify({
      type: "error",
      code: "room_full",
      message: "Room is full",
    }),
  );
  await expect(page.getByRole("alert")).toContainText("Room is full");
  await expect(page.locator("button[type=submit]")).toBeEnabled();
  expect(dialogs).toEqual([]);
  await page.goto("/board.html");
  await expect(page.getByRole("alert")).toBeVisible();
  await expect(page.locator("#arm")).toBeDisabled();
  await expect(page.locator("#reset")).toBeDisabled();
});

test("first buzz sounds once per round and mute persists", async ({ page }) => {
  // Record the audio side effect without depending on an OS audio device.
  await page.addInitScript(() => {
    (window as any).plays = 0;
    HTMLMediaElement.prototype.play = async function () {
      (window as any).plays++;
    };
  });
  const t = await open(page, "/board.html?k=test");
  t.sockets[0].send(JSON.stringify({ type: "helloOk", role: "board" }));
  const first = [{ playerId: "p1", name: "Alex", place: 1 }];
  const second = [...first, { playerId: "p2", name: "Sam", place: 2 }];
  const send = (sequence: any[], roundId = 1) =>
    t.sockets[0].send(
      JSON.stringify(snapshot({ accepting: true, roundId, sequence })),
    );
  send([]);
  send(first);
  await expect.poll(() => page.evaluate(() => (window as any).plays)).toBe(1);
  send(second);
  await expect(page.locator("#list li")).toHaveCount(2);
  expect(await page.evaluate(() => (window as any).plays)).toBe(1);
  await page.getByRole("button", { name: "Mute", exact: true }).click();
  send([], 2);
  send(first, 2);
  await expect(page.locator("#list li")).toHaveCount(1);
  expect(await page.evaluate(() => (window as any).plays)).toBe(1);
  await page.reload();
  await expect(
    page.getByRole("button", { name: "Unmute", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
});

for (const theme of ["light", "dark"]) {
  for (const size of [
    { width: 1920, height: 1080 },
    { width: 1280, height: 720 },
    { width: 360, height: 780 },
    { width: 320, height: 740 },
    { width: 740, height: 360 },
  ]) {
    test(`${theme} layouts stay within viewport at ${size.width}x${size.height}`, async ({
      page,
    }, testInfo) => {
      await page.addInitScript(
        (theme) => localStorage.setItem("quizBuzzer.theme", theme),
        theme,
      );
      await page.setViewportSize(size);
      await page.emulateMedia({ reducedMotion: "reduce" });
      const t = await transport(page);
      const overflow = async () =>
        expect(
          await page.evaluate(
            () => document.documentElement.scrollWidth <= innerWidth,
          ),
        ).toBe(true);
      await page.goto("/board.html?k=test");
      await expect.poll(() => t.sockets.length).toBe(1);
      t.sockets[0].send(JSON.stringify({ type: "helloOk", role: "board" }));
      const sequence = Array.from({ length: 200 }, (_, i) => ({
        playerId: `p${i}`,
        name: i ? `Player ${i + 1}` : "WWWWWWWWWWWWWWWWWWWWWWWW",
        place: i + 1,
      }));
      t.sockets[0].send(
        JSON.stringify(
          snapshot({
            sequence,
            lanUrls: ["http://192.168.1.10:7423/", "http://10.0.0.10:7423/"],
            you: { role: "board" },
          }),
        ),
      );
      await expect(page.locator("#qrs canvas")).toHaveCount(2);
      await expect(page.locator("#list li")).toHaveCount(200);
      const qr = await page.locator("#qrs canvas").first().boundingBox();
      const caption = await page
        .locator("#qrs figcaption")
        .first()
        .boundingBox();
      expect(qr!.y + qr!.height).toBeLessThanOrEqual(caption!.y);
      if (size.width >= 1000) {
        const results = await page.locator(".sequence-panel").boundingBox();
        expect(results!.y + results!.height).toBeLessThanOrEqual(size.height);
      }
      await overflow();
      await expect(page.locator("#list")).toBeVisible();
      expect(
        await page
          .locator("#list")
          .evaluate((el) => el.scrollHeight > el.clientHeight),
      ).toBe(true);
      await page.screenshot({
        path: testInfo.outputPath(`board-${size.width}.png`),
        fullPage: true,
      });
      await page.goto("/player.html");
      await page.locator("#name").fill("Alexandra");
      await overflow();
      await page.screenshot({
        path: testInfo.outputPath(`player-join-${size.width}.png`),
        fullPage: true,
      });
      const ws = t.sockets[t.sockets.length - 1];
      await page.locator("button[type=submit]").click();
      ws.send(
        JSON.stringify({ type: "helloOk", role: "player", playerId: "p1" }),
      );
      ws.send(JSON.stringify(snapshot({ accepting: true })));
      await expect(page.locator("#buzzer")).toBeEnabled();
      await overflow();
      if (size.width === 740) {
        const buzzerBox = await page.locator("#buzzer").boundingBox();
        expect(buzzerBox!.y + buzzerBox!.height).toBeLessThanOrEqual(
          size.height,
        );
      }
      await page.screenshot({
        path: testInfo.outputPath(`player-ready-${size.width}.png`),
        fullPage: true,
      });
    });
  }
}

test("joining explains missing addresses and preserves manual entry when QR generation fails", async ({
  page,
}) => {
  const t = await open(page, "/board.html?k=test");
  t.sockets[0].send(JSON.stringify({ type: "helloOk", role: "board" }));
  t.sockets[0].send(
    JSON.stringify(snapshot({ lanUrls: [], you: { role: "board" } })),
  );
  await expect(page.locator("#qrs")).toContainText("Connect the host to Wi-Fi");
  // Exceed QR capacity to exercise the real encoder's failure path.
  const oversizedAddress = `http://192.168.1.10:7423/${"a".repeat(5000)}`;
  t.sockets[0].send(
    JSON.stringify(
      snapshot({ lanUrls: [oversizedAddress], you: { role: "board" } }),
    ),
  );
  await expect(page.locator("#qrs")).toContainText(
    "QR unavailable. Enter this address on your phone.",
  );
  await expect(page.locator("figcaption")).toContainText(oversizedAddress);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("host console stays visible with a full buzz list in the laptop window", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  const t = await open(page, "/board.html?k=test");
  t.sockets[0].send(JSON.stringify({ type: "helloOk", role: "board" }));
  t.sockets[0].send(
    JSON.stringify(
      snapshot({
        sequence: Array.from({ length: 200 }, (_, i) => ({
          playerId: `p${i}`,
          name: `Player ${i}`,
          place: i + 1,
        })),
        you: { role: "board" },
      }),
    ),
  );
  await expect(page.locator("#list li")).toHaveCount(200);
  for (const id of ["arm", "reset", "theme-toggle", "mute", "toggle-join"]) {
    const box = await page.locator(`#${id}`).boundingBox();
    expect(box!.y).toBeGreaterThanOrEqual(0);
    expect(box!.y + box!.height).toBeLessThanOrEqual(720);
  }
  await page.locator("#list").focus();
  await page.keyboard.press("End");
  await expect
    .poll(() => page.locator("#list").evaluate((el) => el.scrollTop))
    .toBeGreaterThan(0);
});

test("player announces round readiness without repeating unchanged snapshots", async ({
  page,
}) => {
  const t = await open(page, "/player.html");
  await page.locator("#name").fill("Alex");
  await page.locator("#join-button").click();
  t.sockets[0].send(
    JSON.stringify({ type: "helloOk", role: "player", playerId: "p1" }),
  );
  t.sockets[0].send(JSON.stringify(snapshot()));
  const note = page.locator("#play-note");
  await expect(note).toHaveAttribute("role", "status");
  await expect(note).toHaveAttribute("aria-atomic", "true");
  await expect(note).toContainText("Stay ready");
  await note.evaluate((el) => {
    (window as any).noteChanges = 0;
    new MutationObserver(() => (window as any).noteChanges++).observe(el, {
      childList: true,
      characterData: true,
      subtree: true,
    });
  });
  t.sockets[0].send(JSON.stringify(snapshot({ accepting: true })));
  await expect(note).toContainText("You’re live");
  await expect
    .poll(() => page.evaluate(() => (window as any).noteChanges))
    .toBe(1);
  t.sockets[0].send(
    JSON.stringify(
      snapshot({
        accepting: true,
        players: [{ id: "p1", name: "Sam", connected: true }],
      }),
    ),
  );
  await expect(page.locator("#player-name")).toHaveText("Sam");
  expect(await page.evaluate(() => (window as any).noteChanges)).toBe(1);
  t.sockets[0].send(
    JSON.stringify(
      snapshot({
        accepting: true,
        you: { role: "player", id: "p1", place: 2 },
      }),
    ),
  );
  await expect(note).toContainText("Your place is saved");
  t.sockets[0].send(JSON.stringify(snapshot({ accepting: true, roundId: 2 })));
  await expect(note).toContainText("You’re live");
  await expect(page.locator("#buzzer")).toBeEnabled();
});

for (const place of [null, 1]) {
  test(`refresh resumes the same player and restores place ${place}`, async ({
    page,
  }) => {
    const t = await open(page, "/player.html");
    await page.locator("#name").fill("Alex");
    await page.locator("#join-button").click();
    t.sockets[0].send(
      JSON.stringify({ type: "helloOk", role: "player", playerId: "p1" }),
    );
    t.sockets[0].send(
      JSON.stringify(
        snapshot({ accepting: true, you: { role: "player", id: "p1", place } }),
      ),
    );
    await expect(page.locator("#player-name")).toHaveText("Alex");
    t.sent.length = 0;
    await page.reload();
    await expect.poll(() => t.sockets.length).toBe(2);
    await expect
      .poll(() => t.sent)
      .toEqual([
        { type: "hello", role: "player", name: "Alex", playerId: "p1" },
      ]);
    await expect(page.locator("#join")).toBeHidden();
    await expect(page.locator("#buzzer")).toBeDisabled();
    // A snapshot arriving before the resume handshake cannot enable buzzing.
    t.sockets[1].send(JSON.stringify(snapshot({ accepting: true })));
    await expect(page.locator("#buzzer")).toBeDisabled();
    t.sockets[1].send(
      JSON.stringify({ type: "helloOk", role: "player", playerId: "p1" }),
    );
    await expect(page.locator("#buzzer")).toBeDisabled();
    t.sockets[1].send(
      JSON.stringify(
        snapshot({ accepting: true, you: { role: "player", id: "p1", place } }),
      ),
    );
    await expect(page.locator("#player-name")).toHaveText("Alex");
    if (place === null) await expect(page.locator("#buzzer")).toBeEnabled();
    else await expect(page.locator("#place-number")).toHaveText("1st");
  });
}

test("rejected refresh resume returns to a usable name form", async ({
  page,
}) => {
  await page.addInitScript(() => {
    sessionStorage.setItem("quizBuzzer.playerId", "p1");
    sessionStorage.setItem("quizBuzzer.playerName", "Alex");
  });
  const t = await open(page, "/player.html");
  await expect.poll(() => t.sent.length).toBe(1);
  t.sockets[0].send(
    JSON.stringify({
      type: "error",
      code: "room_full",
      message: "Room is full",
    }),
  );
  await expect(page.locator("#join")).toBeVisible();
  await expect(page.locator("#name")).toHaveValue("Alex");
  await expect(page.locator("#join-button")).toBeEnabled();
  await expect(page.locator("#error")).toContainText("Room is full");
  await expect(page.locator("#play")).toBeHidden();
});

test("unavailable session storage does not prevent joining", async ({
  page,
}) => {
  await page.addInitScript(() => {
    Object.defineProperty(window, "sessionStorage", {
      get() {
        throw new DOMException("Blocked", "SecurityError");
      },
    });
  });
  const t = await open(page, "/player.html");
  await page.locator("#name").fill("Alex");
  await page.locator("#join-button").click();
  t.sockets[0].send(
    JSON.stringify({ type: "helloOk", role: "player", playerId: "p1" }),
  );
  t.sockets[0].send(JSON.stringify(snapshot({ accepting: true })));
  await expect(page.locator("#buzzer")).toBeEnabled();
});
