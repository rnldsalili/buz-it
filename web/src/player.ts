import { connect, type ConnectionState } from "./ws";
import { element, onPress, showConnection, showError } from "./ui";

const joinForm = element<HTMLFormElement>("join");
const nameInput = element<HTMLInputElement>("name");
const joinButton = element<HTMLButtonElement>("join-button");
const buzzer = element<HTMLButtonElement>("buzzer");
const placeEl = element("place");
const KEY = "quizBuzzer.playerId";
const NAME_KEY = "quizBuzzer.playerName";
let playerId: string | null = null;
let displayName = "";
try {
  playerId = sessionStorage.getItem(KEY);
  displayName = sessionStorage.getItem(NAME_KEY) ?? "";
} catch {
  // Joining still works when the browser blocks session storage.
}
let joined = Boolean(playerId && displayName.trim());
let authenticated = false;
let ready = false;
let pendingJoin = false;
let connectionState: ConnectionState = "connecting";
let accepting = false;
let place: number | null = null;
nameInput.value = displayName;

function saveSession() {
  if (!playerId) return;
  try {
    sessionStorage.setItem(KEY, playerId);
    sessionStorage.setItem(NAME_KEY, displayName);
  } catch {
    // Keep the live session usable even when persistence is unavailable.
  }
}

function ordinal(n: number): string {
  const v = n % 100;
  if (v >= 11 && v <= 13) return `${n}th`;
  return `${n}${({ 1: "st", 2: "nd", 3: "rd" } as Record<number, string>)[n % 10] ?? "th"}`;
}

function render() {
  joinForm.hidden = joined;
  element("play").hidden = !joined;
  joinButton.disabled = connectionState !== "open" || pendingJoin;
  joinButton.textContent = pendingJoin ? "Joining…" : "Join the game";
  nameInput.disabled = pendingJoin;
  buzzer.hidden = !joined || place !== null;
  placeEl.hidden = !joined || place === null;
  buzzer.disabled = !ready || !accepting;
  element("buzzer-label").textContent = !ready
    ? "Hold on"
    : accepting
      ? "BUZZ"
      : "Wait";
  element("buzzer-hint").textContent = !ready
    ? "Reconnecting to the game…"
    : accepting
      ? "Tap as soon as you know it."
      : "The host will start the round.";
  element("player-name").textContent = displayName;
  const playNote = !ready
    ? "Connection interrupted. Waiting to rejoin the game."
    : place !== null
      ? "Your place is saved for this round."
      : accepting
        ? "You’re live. Make it count."
        : "Stay ready. Your moment is coming.";
  // Keep live announcements quiet when a snapshot does not change our state.
  const playNoteEl = element("play-note");
  if (playNoteEl.textContent !== playNote) playNoteEl.textContent = playNote;
  if (place !== null) {
    element("place-number").textContent = ordinal(place);
    element("place-note").textContent =
      place === 1
        ? "You’re first. The floor is yours."
        : "You’re in the buzz order.";
    placeEl.classList.toggle("winner", place === 1);
  }
}

const sock = connect(
  (msg) => {
    if (msg.type === "helloOk" && msg.role === "player" && msg.playerId) {
      playerId = msg.playerId;
      saveSession();
      authenticated = true;
      joined = true;
      pendingJoin = false;
      render();
    }
    if (msg.type === "error") {
      pendingJoin = false;
      if (!authenticated) {
        // A failed automatic resume must leave a usable manual join form.
        joined = false;
        ready = false;
        place = null;
        playerId = null;
        nameInput.value = displayName;
        try {
          sessionStorage.removeItem(KEY);
          sessionStorage.removeItem(NAME_KEY);
        } catch {
          // Storage is optional for the current connection.
        }
      }
      showError(msg.message);
      render();
    }
    if (
      msg.type === "snapshot" &&
      authenticated &&
      connectionState === "open"
    ) {
      ready = true;
      accepting = msg.accepting;
      place = msg.you.place ?? null;
      displayName =
        msg.players.find((p) => p.id === playerId)?.name ?? displayName;
      saveSession();
      showConnection("ready");
      showError("");
      render();
    }
  },
  () => {
    if (joined)
      sock.send({ type: "hello", role: "player", name: displayName, playerId });
  },
  (state) => {
    connectionState = state;
    ready = false;
    authenticated = false;
    pendingJoin = false;
    showConnection(state);
    render();
  },
);

joinForm.addEventListener("submit", (ev) => {
  ev.preventDefault();
  if (joinButton.disabled) return;
  displayName = nameInput.value.trim();
  if (!displayName) {
    showError("Enter a name to join the game.");
    nameInput.focus();
    return;
  }
  showError("");
  pendingJoin = sock.send({
    type: "hello",
    role: "player",
    name: displayName,
    playerId,
  });
  render();
});
onPress(buzzer, () => {
  if (ready && accepting && place === null) sock.send({ type: "buzz" });
});
