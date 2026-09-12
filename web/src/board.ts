import { toCanvas } from "qrcode";
import { connect } from "./ws";
import type { ServerMessage } from "./protocol";

const MUTE_KEY = "quizBuzzer.mute";
const OPEN_FROM_APP = "Open this board from the Quiz Buzzer app.";

const status = document.getElementById("status") as HTMLParagraphElement;
const main = document.getElementById("main") as HTMLElement;
const list = document.getElementById("list") as HTMLOListElement;
const qrs = document.getElementById("qrs") as HTMLElement;
const mute = document.getElementById("mute") as HTMLButtonElement;

const k = new URLSearchParams(location.search).get("k") ?? "";
const lockoutAudio = new Audio("/lockout.wav");
lockoutAudio.preload = "auto";
lockoutAudio.load();

let lastRoundId: number | null = null;
let lastSeqLen = 0;
let lastLanKey = "";
let audioUnlocked = false;

function isMuted(): boolean {
  return localStorage.getItem(MUTE_KEY) === "1";
}

function renderMute() {
  mute.textContent = isMuted() ? "Unmute" : "Mute";
}

function renderSnapshot(msg: Extract<ServerMessage, { type: "snapshot" }>) {
  if (lastRoundId !== msg.roundId) {
    lastRoundId = msg.roundId;
    lastSeqLen = 0;
  }
  if (lastSeqLen === 0 && msg.sequence.length >= 1 && !isMuted()) {
    lockoutAudio.currentTime = 0;
    void lockoutAudio.play().catch(() => {});
  }
  lastSeqLen = msg.sequence.length;

  const n = msg.players.filter((p) => p.connected).length;
  status.textContent = `${msg.accepting ? "ARMED" : "LOCKED"} · ${n} players`;
  status.classList.toggle("armed", msg.accepting);
  status.classList.toggle("locked", !msg.accepting);

  if (msg.sequence.length > 0) {
    main.textContent = msg.sequence[0].name;
  } else if (msg.accepting) {
    main.textContent = "BUZZ";
  } else {
    main.textContent = "";
  }

  list.replaceChildren(
    ...msg.sequence.map((place) => {
      const li = document.createElement("li");
      li.textContent = place.name;
      return li;
    }),
  );

  const lanKey = msg.lanUrls.join("\n");
  if (lanKey !== lastLanKey) {
    lastLanKey = lanKey;
    void renderQrs(msg.lanUrls);
  }
}

async function renderQrs(urls: string[]) {
  qrs.replaceChildren();
  for (const item of urls) {
    const lan = document.createElement("div");
    lan.className = "lan";

    const playerFig = document.createElement("figure");
    const playerCanvas = document.createElement("canvas");
    const playerCap = document.createElement("figcaption");
    playerCap.textContent = item;
    playerFig.append(playerCanvas, playerCap);
    await toCanvas(playerCanvas, item, { width: 220, margin: 1 });

    const clickerFig = document.createElement("figure");
    clickerFig.className = "clicker";
    const clickerCanvas = document.createElement("canvas");
    const clickerCap = document.createElement("figcaption");
    clickerCap.textContent = "Host clicker";
    clickerFig.append(clickerCanvas, clickerCap);
    await toCanvas(clickerCanvas, new URL("host?k=" + k, item).toString(), {
      width: 160,
      margin: 1,
    });

    lan.append(playerFig, clickerFig);
    qrs.append(lan);
  }
}

renderMute();
mute.addEventListener("pointerdown", () => {
  if (audioUnlocked) return;
  audioUnlocked = true;
  lockoutAudio.load();
});
mute.addEventListener("click", () => {
  localStorage.setItem(MUTE_KEY, isMuted() ? "0" : "1");
  renderMute();
});

if (!k) {
  main.textContent = OPEN_FROM_APP;
} else {
  const sock = connect(
    (msg: ServerMessage) => {
      if (msg.type === "error") {
        main.textContent = OPEN_FROM_APP;
        return;
      }
      if (msg.type === "snapshot") {
        renderSnapshot(msg);
      }
    },
    () => {
      sock.send({ type: "hello", role: "board", hostKey: k });
    },
  );
}
