import type { ClientMessage, ServerMessage } from "./protocol";

export type ConnectionState = "connecting" | "open" | "reconnecting" | "closed";

export function connect(
  onMessage: (msg: ServerMessage) => void,
  onOpen: () => void,
  onState: (state: ConnectionState) => void = () => {},
): { send: (msg: ClientMessage) => boolean; close: () => void } {
  const url = `${location.protocol === "https:" ? "wss:" : "ws:"}//${location.host}/ws`;
  let ws: WebSocket;
  let closed = false;
  let retry: ReturnType<typeof setTimeout> | undefined;
  onState("connecting");

  const bind = () => {
    ws = new WebSocket(url);
    ws.addEventListener("open", () => {
      onState("open");
      onOpen();
    });
    ws.addEventListener("message", (ev) => {
      onMessage(JSON.parse(String(ev.data)) as ServerMessage);
    });
    ws.addEventListener("close", () => {
      if (closed) return;
      onState("reconnecting");
      retry = setTimeout(() => {
        if (!closed) bind();
      }, 400);
    });
    ws.addEventListener("error", () => {
      if (!closed) onState("reconnecting");
    });
  };
  bind();

  return {
    send(msg) {
      if (closed || ws.readyState !== WebSocket.OPEN) return false;
      ws.send(JSON.stringify(msg));
      return true;
    },
    close() {
      closed = true;
      clearTimeout(retry);
      onState("closed");
      ws.close();
    },
  };
}
