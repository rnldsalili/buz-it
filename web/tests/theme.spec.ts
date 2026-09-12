import { test, expect } from "@playwright/test";

for (const path of ["/board.html?k=test", "/player.html"]) {
  test(`light default and saved dark preference on ${path}`, async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.routeWebSocket("**/ws", () => {});
    await page.goto(path);
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
    await expect(page).toHaveTitle(/Buz It/);
    const theme = page.getByRole("switch", { name: "Dark mode" });
    await expect(theme).toHaveAttribute("aria-checked", "false");
    await theme.click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    await expect(theme).toHaveAttribute("aria-checked", "true");
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    await theme.focus();
    await page.keyboard.press("Space");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  });
}

test("board controls require authentication and a fresh snapshot; theme changes do not send game commands", async ({
  page,
}) => {
  const messages: any[] = [];
  const sockets: any[] = [];
  await page.routeWebSocket("**/ws", (ws) => {
    sockets.push(ws);
    ws.onMessage((m) => messages.push(JSON.parse(String(m))));
  });
  await page.goto("/board.html?k=test");
  const arm = page.getByRole("button", { name: "Start round", exact: true });
  const close = page.getByRole("button", {
    name: "Close buzzing",
    exact: true,
  });
  await expect(arm).toBeDisabled();
  const snap = {
    type: "snapshot",
    roundId: 0,
    accepting: false,
    sequence: [],
    players: [],
    lanUrls: ["http://192.168.1.10:7423/"],
    you: { role: "board" },
  };
  sockets[0].send(JSON.stringify(snap));
  await expect(arm).toBeDisabled();
  sockets[0].send(JSON.stringify({ type: "helloOk", role: "board" }));
  await expect(arm).toBeDisabled();
  sockets[0].send(JSON.stringify(snap));
  await expect(arm).toBeEnabled();
  await expect(page.locator("#qrs canvas")).toHaveCount(1);
  await page.getByRole("switch", { name: "Dark mode" }).click();
  expect(messages.filter((m) => m.type !== "hello")).toEqual([]);
  await arm.click();
  await expect
    .poll(() => messages.filter((m) => m.type === "arm").length)
    .toBe(1);
  sockets[0].close();
  await expect(arm).toBeDisabled();
  await expect(close).toBeDisabled();
  await expect.poll(() => sockets.length).toBe(2);
  sockets[1].send(JSON.stringify(snap));
  await expect(arm).toBeDisabled();
  sockets[1].send(JSON.stringify({ type: "helloOk", role: "board" }));
  sockets[1].send(JSON.stringify(snap));
  await expect(close).toBeEnabled();
  await close.click();
  await expect
    .poll(() => messages.filter((m) => m.type === "reset").length)
    .toBe(1);
});

test("saved theme is applied before page modules load", async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.setItem("quizBuzzer.theme", "dark"),
  );
  // The game module cannot apply a late theme on our behalf.
  await page.route("**/src/player.ts", (route) => route.abort());
  await page.goto("/player.html");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  expect(
    await page
      .locator("html")
      .evaluate((el) => getComputedStyle(el).colorScheme),
  ).toBe("dark");
});

test("theme remains usable without browser storage", async ({ page }) => {
  await page.addInitScript(() => {
    Storage.prototype.getItem = () => {
      throw new DOMException("Denied", "SecurityError");
    };
    Storage.prototype.setItem = () => {
      throw new DOMException("Denied", "SecurityError");
    };
  });
  await page.routeWebSocket("**/ws", () => {});
  await page.goto("/player.html");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page.getByRole("switch", { name: "Dark mode" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
});

test("theme switching preserves a player’s live buzzer and place", async ({
  page,
}) => {
  let socket: any;
  const messages: any[] = [];
  await page.routeWebSocket("**/ws", (ws) => {
    socket = ws;
    ws.onMessage((m) => messages.push(JSON.parse(String(m))));
  });
  await page.goto("/player.html");
  await page.locator("#name").fill("Alex");
  await page.locator("#join-button").click();
  socket.send(
    JSON.stringify({ type: "helloOk", role: "player", playerId: "p1" }),
  );
  const snap = {
    type: "snapshot",
    roundId: 1,
    accepting: true,
    sequence: [],
    players: [{ id: "p1", name: "Alex", connected: true }],
    lanUrls: [],
    you: { role: "player", id: "p1", place: null },
  };
  socket.send(JSON.stringify(snap));
  await expect(page.locator("#buzzer")).toBeEnabled();
  await page.getByRole("switch", { name: "Dark mode" }).click();
  await expect(page.locator("#buzzer")).toBeEnabled();
  expect(messages.filter((m) => m.type !== "hello")).toEqual([]);
  await page.locator("#buzzer").click();
  await expect
    .poll(() => messages.filter((m) => m.type === "buzz").length)
    .toBe(1);
  socket.send(
    JSON.stringify({
      ...snap,
      sequence: [{ playerId: "p1", name: "Alex", place: 1 }],
      you: { ...snap.you, place: 1 },
    }),
  );
  await expect(page.locator("#place")).toContainText("1st");
  await page.getByRole("switch", { name: "Dark mode" }).click();
  await expect(page.locator("#place")).toContainText("1st");
  await expect(page.locator("#buzzer")).toBeHidden();
  expect(messages.filter((m) => m.type === "buzz")).toHaveLength(1);
});
