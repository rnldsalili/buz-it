import { connect } from "./ws";
import type { ServerMessage } from "./protocol";

const OPEN_FROM_QR = "Open this page from the board Host clicker QR.";

const status = document.getElementById("status") as HTMLParagraphElement;
const arm = document.getElementById("arm") as HTMLButtonElement;
const reset = document.getElementById("reset") as HTMLButtonElement;

const k = new URLSearchParams(location.search).get("k") ?? "";

if (!k) {
  status.textContent = OPEN_FROM_QR;
  arm.disabled = true;
  reset.disabled = true;
} else {
  const sock = connect(
    (msg: ServerMessage) => {
      if (msg.type === "error") {
        alert(msg.message);
        return;
      }
      if (msg.type === "snapshot") {
        const connectedCount = msg.players.filter((p) => p.connected).length;
        const firstName = msg.sequence[0]?.name ?? "—";
        status.textContent = `${msg.accepting ? "ARMED" : "LOCKED"} · ${connectedCount} players · #1 ${firstName}`;
      }
    },
    () => {
      sock.send({ type: "hello", role: "clicker", hostKey: k });
    },
  );

  function sendArm(ev: Event) {
    ev.preventDefault();
    sock.send({ type: "arm" });
  }

  function sendReset(ev: Event) {
    ev.preventDefault();
    sock.send({ type: "reset" });
  }

  arm.addEventListener("pointerdown", sendArm);
  reset.addEventListener("pointerdown", sendReset);
}
