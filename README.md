# Quiz Buzzer

Local LAN quiz buzzer. The host laptop runs a Tauri app (or the core server); phones on the **same Wi‑Fi** join in the browser and tap to buzz. The TV board shows who was first and the full sequence.

This is **not** a cloud app. Nothing is deployed to Cloudflare or the public internet. It is **not** for remote play — everyone must be on the same local network.

## Requirements

Phones must reach the laptop on TCP port **7423** (override with `QUIZ_BUZZER_PORT`).

- Same Wi‑Fi / SSID as the host
- **Not** a guest network
- Access-point **client isolation** must be off
- Allow the firewall prompt on first launch (or open port 7423)

If the board loads on the laptop but phones never connect, it is almost always isolation or the wrong network.

## Run (dev)

Install deps at the repo root and in `web/`:

```bash
npm install
npm install --prefix web
npm run build:ui
```

Then either:

```bash
cargo tauri dev
```

or, after the UI build:

```bash
cargo run -p quiz-buzzer-core --bin server
```

Default listen port is **7423**. Override with env `QUIZ_BUZZER_PORT`.

## How to operate

- **Board** — the laptop window (put it on the TV). Shows armed/locked status, first buzz, ranked list, and join QR codes.
- **Host clicker** — scan the **small** QR labeled **Host clicker**. Use **ARM** to start a round and **RESET** to freeze the list.
- **Players** — scan the **big** QR, type a name, then tap the buzzer (fires on pointer down).

Ranking is server arrival order on the host process. One running app = one room.

## Docs

| Document | Path |
|---|---|
| Design (product, protocol, architecture) | [`docs/superpowers/specs/2026-09-13-quiz-buzzer-design.md`](docs/superpowers/specs/2026-09-13-quiz-buzzer-design.md) |
| Implementation plan (task-by-task) | [`docs/superpowers/plans/2026-09-13-quiz-buzzer.md`](docs/superpowers/plans/2026-09-13-quiz-buzzer.md) |

## Stack

Rust (`quiz-buzzer-core` + Axum WebSocket) + Vite/TypeScript pages, wrapped in **Tauri 2** so the quizmaster can run a window instead of a terminal.
