import type { ConnectionState } from "./ws";

export const element = <T extends HTMLElement = HTMLElement>(id: string): T => {
  const node = document.getElementById(id);
  if (!node) throw new Error(`Missing UI element: ${id}`);
  return node as T;
};

export function showError(message: string) {
  const error = element("error");
  error.textContent = message;
  error.hidden = !message;
}

export function showConnection(state: ConnectionState | "ready") {
  const connection = element("connection");
  connection.dataset.state = state;
  connection.textContent = {
    connecting: "Connecting…",
    open: "Connecting…",
    reconnecting: "Reconnecting…",
    closed: "Disconnected",
    ready: "Connected",
  }[state];
}

// Pointer activation stays immediate. Keyboard activation must not also generate
// a second command via the browser's synthetic click.
export function onPress(button: HTMLButtonElement, action: () => void) {
  const press = (ev: Event) => {
    ev.preventDefault();
    if (!button.disabled) action();
  };
  button.addEventListener("pointerdown", (ev) => {
    if (ev.button === 0 && ev.isPrimary) press(ev);
  });
  button.addEventListener("keydown", (ev) => {
    if (ev.code === "Space" || ev.code === "Enter") {
      ev.preventDefault();
      if (!ev.repeat) press(ev);
    }
  });
  // Assistive technology can activate a button without pointer or key events.
  button.addEventListener("click", (ev) => {
    if (ev.detail === 0) press(ev);
  });
}
