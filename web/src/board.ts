import { toCanvas } from "qrcode";
import { connect } from "./ws";
import { element, onPress, showConnection, showError } from "./ui";
import type { ServerMessage } from "./protocol";

const MUTE_KEY = "buzIt.mute";
const OPEN_FROM_APP = "Open this board from the Buz It app.";
const status = element("status");
const main = element("main");
const list = element<HTMLOListElement>("list");
const qrs = element("qrs");
const mute = element<HTMLButtonElement>("mute");
const toggleJoin = element<HTMLButtonElement>("toggle-join");
const panel = element("join-panel");
const arm = element<HTMLButtonElement>("arm");
const reset = element<HTMLButtonElement>("reset");
let ready = false;
const spotlight = element("spotlight");
const waitingMark = element("winner-mark").innerHTML;
const k = new URLSearchParams(location.search).get("k") ?? "";
const lockoutAudio = new Audio("/lockout.wav");
lockoutAudio.preload = "auto";
lockoutAudio.load();
let lastRoundId: number | null = null;
let lastSeqLen = 0;
let lastLanKey: string | null = null;
let qrVersion = 0;
let authenticated = false;
let audioUnlocked = false;

function isMuted() {
  return localStorage.getItem(MUTE_KEY) === "1";
}
function renderMute() {
  mute.textContent = isMuted() ? "Unmute" : "Mute";
  mute.setAttribute("aria-pressed", String(isMuted()));
}

function renderSnapshot(msg: Extract<ServerMessage, { type: "snapshot" }>) {
  if (lastRoundId !== msg.roundId) {
    lastRoundId = msg.roundId;
    lastSeqLen = 0;
    spotlight.classList.remove("reveal");
  }
  const firstBuzz = lastSeqLen === 0 && msg.sequence.length >= 1;
  if (firstBuzz) {
    spotlight.classList.add("reveal");
    if (!isMuted()) {
      lockoutAudio.currentTime = 0;
      void lockoutAudio.play().catch(() => {});
    }
  }
  lastSeqLen = msg.sequence.length;
  const count = msg.players.filter((p) => p.connected).length;
  element("player-count").textContent =
    `${count} ${count === 1 ? "player" : "players"} connected`;
  status.textContent = msg.accepting ? "Buzzing open" : "Buzzing closed";
  status.classList.toggle("armed", msg.accepting);
  status.classList.toggle("locked", !msg.accepting);
  spotlight.classList.toggle("winner", msg.sequence.length > 0);
  if (msg.sequence.length) {
    main.textContent = msg.sequence[0].name;
    main.classList.toggle("long-name", [...msg.sequence[0].name].length > 16);
    element("spotlight-label").textContent =
      "First to buzz. The floor is yours.";
    element("winner-mark").textContent = "01";
    element("main-note").textContent = msg.accepting
      ? "Buzzing is still open for everyone else."
      : "Buzzing is closed. These results are saved.";
  } else {
    main.classList.remove("long-name");
    main.textContent = msg.accepting ? "Thumbs ready?" : "Everyone in?";
    element("spotlight-label").textContent = msg.accepting
      ? "The round is live. Go for it."
      : "Find your seat. Bring your best answers.";
    element("winner-mark").innerHTML = waitingMark;
    element("main-note").textContent = msg.accepting
      ? "Know the answer? Hit your buzzer."
      : "Join on your phone. The host will start the round.";
  }
  element("empty-list").hidden = msg.sequence.length > 0;
  element("sequence-count").textContent =
    `${msg.sequence.length} ${msg.sequence.length === 1 ? "buzz" : "buzzes"}`;
  list.replaceChildren(
    ...msg.sequence.map((place) => {
      const li = document.createElement("li");
      const rank = document.createElement("span");
      rank.className = "rank";
      rank.textContent = String(place.place).padStart(2, "0");
      const name = document.createElement("span");
      name.className = "player-name";
      name.textContent = place.name;
      li.append(rank, name);
      if (place.place === 1) {
        const tag = document.createElement("span");
        tag.className = "first-tag";
        tag.textContent = "FIRST";
        li.append(tag);
      }
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
  const version = ++qrVersion;
  const content = document.createDocumentFragment();
  if (!urls.length) {
    const note = document.createElement("p");
    note.className = "muted";
    note.textContent =
      "No LAN address found. Connect the host to Wi-Fi to show join codes.";
    content.append(note);
  }
  for (const item of urls) {
    const lan = document.createElement("div");
    lan.className = "lan";
    const playerFig = document.createElement("figure");
    const playerCanvas = document.createElement("canvas");
    playerCanvas.setAttribute("role", "img");
    playerCanvas.setAttribute("aria-label", `Player join QR code for ${item}`);
    const playerCap = document.createElement("figcaption");
    playerCap.textContent = item;
    playerFig.append(playerCanvas, playerCap);
    try {
      await toCanvas(playerCanvas, item, { width: 260, margin: 4 });
      playerCanvas.removeAttribute("style");
    } catch {
      playerCap.append(" · QR unavailable. Enter this address on your phone.");
    }
    lan.append(playerFig);
    content.append(lan);
  }
  if (version === qrVersion) qrs.replaceChildren(content);
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
toggleJoin.addEventListener("click", () => {
  panel.hidden = !panel.hidden;
  toggleJoin.textContent = panel.hidden ? "Show join codes" : "Hide join codes";
  toggleJoin.setAttribute("aria-expanded", String(!panel.hidden));
  element("board-layout").classList.toggle("join-collapsed", panel.hidden);
});

if (!k) {
  showError(OPEN_FROM_APP);
  showConnection("closed");
} else {
  const sock = connect(
    (msg) => {
      if (msg.type === "helloOk" && msg.role === "board") authenticated = true;
      if (msg.type === "error") {
        showError(msg.message);
        sock.close();
      }
      if (msg.type === "snapshot" && authenticated) {
        ready = true;
        arm.disabled = reset.disabled = false;
        showConnection("ready");
        showError("");
        renderSnapshot(msg);
      }
    },
    () => {
      sock.send({ type: "hello", role: "board", hostKey: k });
    },
    (state) => {
      authenticated = ready = false;
      arm.disabled = reset.disabled = true;
      showConnection(state);
    },
  );
  onPress(arm, () => {
    if (ready) sock.send({ type: "arm" });
  });
  onPress(reset, () => {
    if (ready) sock.send({ type: "reset" });
  });
}
