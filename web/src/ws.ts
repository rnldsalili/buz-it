import type { ClientMessage, ServerMessage } from "./protocol";

export function connect(
  onMessage: (msg: ServerMessage) => void,
  onOpen: () => void,
): { send: (msg: ClientMessage) => void; close: () => void } {
  const url = `${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/ws`;
  let ws = new WebSocket(url);
  let closed = false;

  const bind = () => {
    ws.addEventListener("open", onOpen);
    ws.addEventListener("message", (ev) => {
      const msg = JSON.parse(String(ev.data)) as ServerMessage;
      onMessage(msg);
    });
    ws.addEventListener("close", () => {
      if (closed) return;
      setTimeout(() => {
        if (closed) return;
        ws = new WebSocket(url);
        bind();
      }, 400);
    });
  };
  bind();

  return {
    send(msg) {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(msg));
      }
    },
    close() {
      closed = true;
      ws.close();
    },
  };
}
