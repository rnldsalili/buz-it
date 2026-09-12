# Quiz Buzzer

Local LAN quiz buzzer. The host laptop runs a Tauri app; phones on the **same Wi‑Fi** join in the browser and tap to buzz. The TV board shows who was first and the full sequence.

This is **not** a cloud app. Nothing is deployed to Cloudflare or the public internet.

## Status

Design and implementation plan only. Code is not built yet.

| Document | Path |
|---|---|
| Design (product, protocol, architecture) | [`docs/superpowers/specs/2026-09-13-quiz-buzzer-design.md`](docs/superpowers/specs/2026-09-13-quiz-buzzer-design.md) |
| Implementation plan (task-by-task) | [`docs/superpowers/plans/2026-09-13-quiz-buzzer.md`](docs/superpowers/plans/2026-09-13-quiz-buzzer.md) |

## What v1 is

- **Board** — laptop window on a TV: first name, ranked list, lockout sound, join QR
- **Clicker** — host phone: ARM / RESET
- **Player** — everyone else: name, full-screen buzzer, then `1st` / `2nd` / …
- Ranking is **server arrival order** on the host process
- One running app = one room

## Network (read this)

Phones must reach the laptop on TCP port **7423**.

- Same SSID as the host
- **Not** a guest network
- Access-point **client isolation** must be off
- Allow the firewall prompt on first launch

If the board loads on the laptop but phones never connect, it is almost always isolation or the wrong network.

## Stack

Rust (`quiz-buzzer-core` + Axum WebSocket) + Vite/TypeScript pages, wrapped in **Tauri 2** so the quizmaster double-clicks instead of running a terminal.
