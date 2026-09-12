import { connect } from "./ws";
import type { ServerMessage } from "./protocol";

const joinForm = document.getElementById("join") as HTMLFormElement;
const nameInput = document.getElementById("name") as HTMLInputElement;
const buzzer = document.getElementById("buzzer") as HTMLButtonElement;
const placeEl = document.getElementById("place") as HTMLParagraphElement;

const KEY = "quizBuzzer.playerId";
let playerId = sessionStorage.getItem(KEY);
let joined = false;

function ordinal(n: number): string {
  const v = n % 100;
  if (v >= 11 && v <= 13) return `${n}th`;
  switch (n % 10) {
    case 1:
      return `${n}st`;
    case 2:
      return `${n}nd`;
    case 3:
      return `${n}rd`;
    default:
      return `${n}th`;
  }
}

function showBuzzer(accepting: boolean, place: number | null | undefined) {
  joinForm.hidden = true;
  if (place) {
    buzzer.hidden = true;
    placeEl.hidden = false;
    placeEl.textContent = ordinal(place);
    return;
  }
  placeEl.hidden = true;
  buzzer.hidden = false;
  buzzer.disabled = !accepting;
  buzzer.textContent = accepting ? "BUZZ" : "Wait";
}

const sock = connect(
  (msg: ServerMessage) => {
    if (msg.type === "helloOk" && msg.playerId) {
      playerId = msg.playerId;
      sessionStorage.setItem(KEY, playerId);
      joined = true;
    }
    if (msg.type === "error") {
      alert(msg.message);
    }
    if (msg.type === "snapshot" && joined) {
      showBuzzer(msg.accepting, msg.you.place ?? null);
    }
  },
  () => {
    if (!joined) return;
    sock.send({
      type: "hello",
      role: "player",
      name: nameInput.value,
      playerId,
    });
  },
);

joinForm.addEventListener("submit", (e) => {
  e.preventDefault();
  sock.send({
    type: "hello",
    role: "player",
    name: nameInput.value,
    playerId,
  });
});

function buzz(ev: Event) {
  ev.preventDefault();
  if (buzzer.disabled) return;
  sock.send({ type: "buzz" });
}
buzzer.addEventListener("pointerdown", buzz);
buzzer.addEventListener("keydown", (e) => {
  if (e.code === "Space" || e.code === "Enter") buzz(e);
});
