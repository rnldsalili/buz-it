# Buz It Design

Local, LAN-only web quiz buzzer. The host runs a Tauri desktop app on a laptop plugged into a TV. Players and a host “clicker” join from phones on the same Wi‑Fi. The board shows who tapped first and the full arrival sequence.

This document is the source of truth for product, architecture, protocol, and UI. The bite-sized engineering plan is `docs/superpowers/plans/2026-09-13-buz-it.md`.

## Goal

Ship a double-click host app that starts a LAN server, opens a TV board, and fairly ranks phone buzzes with sub-frame server-side ordering.

## Product (locked)

| Decision | Choice |
|---|---|
| v1 scope | Buzzer + host controls (arm / reset). No question bank, no scoring, no accounts |
| Setting | Same-room LAN only. No cloud, no Cloudflare, no public URL |
| Scale | One room per process; 2–200 devices |
| Round rules | Host arms → first tap wins the floor → later taps still join the sequence → host resets |
| Surfaces | **Board** (TV via Tauri window), **clicker** (host phone), **player buzzer** (everyone else) |
| Identity | Display name only. Duplicate names allowed |
| Join | QR + LAN URL. No short room code (one process = one room) |
| Player phone after tap | That player’s place in line (`1st`, `2nd`, …), not the full list |
| Board audio | Lockout sound on the **first** buzz of a round. Later taps silent. Mute toggle on the board |
| Ranking | Server arrival order. No client clocks, no latency compensation |

## Non-goals (v1)

- Cloudflare, Durable Objects, PartyKit, hosted signaling
- Remote / multi-network fairness
- WASM or all-Rust UI (phones load HTML/JS)
- Native iOS/Android apps, Tauri Mobile
- Accounts, avatars, colors, leaderboards, question content
- Kick / ban, multiple rooms in one process, mDNS (`quiz.local`)
- HTTPS / local CA (phones use `http://192.168.x.x`)
- Packaging notarization beyond a local `tauri build`

## Why this shape

A web buzzer is not a hardware lockout strip. Fairness is a race across touch handling, Wi‑Fi, and the host process.

On a LAN, WebSocket RTT is typically a few milliseconds. The ranking we can defend is **which message the host process handles first**. Client timestamps are cheatable and clocks are skewed. Clock-offset compensation is for a later remote mode, which we are not building.

Rust does not make taps feel fairer. It does give a single host binary and a tight state machine. Tauri is the quizmaster shell: double-click, board window, QR, no terminal. Player and clicker stay mobile browsers because “scan and buzz” beats sideloading.

Node/Bun would work on the hot path. We still pick Rust + Tauri for distribution and host UX.

## Architecture

```
buz-it.app (Tauri 2)
├─ Webview  →  http://127.0.0.1:<port>/board?k=<host_key>
└─ Rust process
   ├─ buz-it-core
   │   ├─ Room state machine (single-threaded via one actor)
   │   ├─ JSON WebSocket protocol
   │   └─ Axum HTTP + WS bound to 0.0.0.0:<port>
   └─ LAN URL + QR data shown on the board
            │
            ├─ board     127.0.0.1 (Tauri window)
            ├─ clicker   http://<lan-ip>:<port>/host?k=<host_key>
            └─ players   http://<lan-ip>:<port>/
```

One process = one room. All three surfaces speak the **same** WebSocket protocol to Axum. The Tauri window is not a second IPC path; it is a browser that happens to load loopback.

### Crate layout

```
buz-it/
  Cargo.toml                  workspace
  crates/core/                buz-it-core (logic + HTTP/WS, no Tauri)
  src-tauri/                  Tauri 2 host (starts core, opens board)
  web/                        Vite + TypeScript UI (three pages)
  docs/                       this spec + implementation plan
```

`buz-it-core` is unit-testable without a GUI. Tauri’s only jobs are: pick a port, generate a host key, spawn the server, discover LAN IPv4 addresses, open `/board?k=...`.

### Bindings and ports

- Default port: `7423` (override with env `BUZ_IT_PORT`)
- Bind: `0.0.0.0:7423` so phones can connect
- Board always uses `http://127.0.0.1:7423` (secure context for autoplay)
- Phones use `http://<lan-ip>:7423` (not a secure context; see Constraints)

### How a buzz is ordered

1. Player `pointerdown` (not `click`) sends one JSON WebSocket text frame: `{"type":"buzz"}`.
2. Axum reads the frame and sends a `Command::Buzz` on an `mpsc` channel to the **room actor**.
3. The actor is the only task that mutates `Room`. Arrival order = channel order.
4. If the round is accepting and this player is not already in the sequence, append and broadcast a snapshot to every socket.
5. Persistence is in-memory only. Kill the app, the room is gone.

Do not `await` anything else on the buzz path (disk, extra locks, HTTP). Broadcast after the in-memory append.

## File map

| Path | Responsibility |
|---|---|
| `crates/core/src/room.rs` | `Room` state machine: hello, buzz, arm, reset, disconnect |
| `crates/core/src/protocol.rs` | JSON message types shared with the frontend conceptually |
| `crates/core/src/actor.rs` | `mpsc` actor + `broadcast` snapshots |
| `crates/core/src/lan.rs` | List non-loopback IPv4 addresses |
| `crates/core/src/server.rs` | Axum router, static files, `/ws` |
| `crates/core/src/lib.rs` | Public `start_server(...)` |
| `src-tauri/src/lib.rs` | Generate host key, start server, open board URL |
| `web/src/protocol.ts` | Matching TS unions for messages |
| `web/src/ws.ts` | Reconnecting WebSocket helper |
| `web/src/player.ts` | Name gate + buzzer |
| `web/src/board.ts` | Sequence, QR, first-buzz sound |
| `web/src/host.ts` | Arm / reset clicker |
| `web/player.html` `board.html` `host.html` | Three entries |

## State machine

```text
                    arm
         ┌──────────────────────────┐
         │                          │
         ▼                          │
   ┌──────────┐    first buzz    ┌──┴────────┐
   │ Idle     │                 │ Accepting │
   │ accepting│   arm (clears)  │ sequence  │
   │ = false  │◄──── reset ─────│ grows     │
   └──────────┘                 └───────────┘
```

- **Idle** (`accepting = false`): buzzes ignored. Last sequence stays on the board so the room can still see who won.
- **Arm**: `accepting = true`, `sequence` cleared, `round_id += 1`. Board shows ready.
- **Buzz** while accepting: first time a given `player_id` buzzes this round, append. Place = `sequence.len()` (1-indexed). Repeat buzzes from the same player ignored.
- **Reset**: `accepting = false`. Sequence **kept**.
- Arm while already accepting: same as arm (new round, clear sequence).

### Data

```rust
PlayerId = Uuid

Player {
  id: PlayerId,
  name: String,       // trimmed, 1..=24 chars, Unicode scalar length
  connected: bool,
}

Room {
  host_key: String,   // 32 hex chars (16 random bytes)
  accepting: bool,
  round_id: u64,      // increments on arm
  players: HashMap<PlayerId, Player>,
  sequence: Vec<PlayerId>,
}
```

`Snapshot` (what every client gets):

```json
{
  "type": "snapshot",
  "accepting": false,
  "roundId": 3,
  "players": [{"id": "...", "name": "Asha", "connected": true}],
  "sequence": [{"playerId": "...", "name": "Asha", "place": 1}],
  "lanUrls": ["http://192.168.1.20:7423/"],
  "you": { "id": "...", "role": "player", "place": 1 }
}
```

`you.place` is `null` until that player is in `sequence`. Boards and clickers still receive `you` with their role.

## Protocol

WebSocket URL: `ws://<host>:7423/ws`

Text frames, JSON, one object per frame. Unknown `type` values: ignore (forward compatible).

### Client → server

```json
{"type":"hello","role":"player","name":"Asha","playerId":null}
{"type":"hello","role":"board","hostKey":"<32 hex>"}
{"type":"hello","role":"clicker","hostKey":"<32 hex>"}
{"type":"buzz"}
{"type":"arm"}
{"type":"reset"}
```

- `playerId` on hello: if the client has a previous id in `sessionStorage` and the server still knows it, resume that player (name may update, `connected = true`). Otherwise the server assigns a new Uuid and returns it.
- `buzz` / `arm` / `reset` before a successful hello: close with policy violation (1008).
- `arm` / `reset` from a non-clicker: ignore.
- `buzz` from a non-player: ignore.

### Server → client

```json
{"type":"helloOk","playerId":"...","role":"player"}
{"type":"error","code":"bad_host_key","message":"Invalid host key"}
{"type":"error","code":"bad_name","message":"Name required"}
{"type":"snapshot", "...": "..."}
```

After `helloOk`, the server immediately sends a `snapshot`. Every successful arm, reset, buzz-accept, join, and disconnect broadcasts a new `snapshot` to **all** connections.

Buzz rejects (`NotAccepting`, `AlreadyBuzzed`) do **not** broadcast. The buzzing client may receive the current snapshot again (cheap) so its UI stays aligned; no separate nack type in v1.

### Auth

- Host key generated at process start (`rand` 16 bytes, hex).
- Printed in the Tauri window title area via the board page (the board itself needed the key to connect).
- Clicker URL includes `?k=<hostKey>`.
- Board URL includes `?k=<hostKey>`.
- Players never see the host key on their page.
- Knowing the key on a LAN is enough to clicker. That is acceptable for a pub-quiz tool.

### Limits

| Limit | Value |
|---|---|
| Display name | trim; reject empty; max 24 Unicode chars |
| Players | 250 connected players; extra hellos get `error` `room_full` |
| Boards / clickers | no extra cap (they are few) |
| Message size | 4 KiB; larger frames dropped |
| Hello timeout | if no hello within 5s, close 1008 |

## Networking and operations

### LAN address

Enumerate IPv4, skip loopback and link-local (`169.254.0.0/16`). Prefer `10/8`, `172.16/12`, `192.168/16`. If several, the board shows **all** of them as URLs + QR (common with VPN + Wi‑Fi). The working one is the Wi‑Fi address; the host picks with their eyes.

### Guest Wi‑Fi / client isolation

Many “guest” SSIDs block phone-to-laptop traffic. Symptom: board loads on the laptop, phones spin on connect. Fix: same SSID as the host, **not** guest, AP client isolation off. Document this in the README and on the board if no player has connected after arm.

### Firewall

First launch on macOS/Windows may prompt to allow inbound TCP. The host must allow it. Bind `0.0.0.0`.

### Secure context

| Surface | Origin | Secure? |
|---|---|---|
| Board in Tauri | `http://127.0.0.1` | yes |
| Clicker / player | `http://192.168.x.x` | no |

`navigator.vibrate` and Screen Wake Lock may be missing on phones. Tap still works. Do not block the buzzer on haptics. Optional vibrate when the API exists.

### Deploys / restarts

Killing the app drops every socket and all state. There is no resume across process restarts. `sessionStorage` player ids only help reconnects while the process lives.

## UI

Shared visual language: dark background, high contrast, large type that reads from a couch. No marketing site, no settings jungle.

### Player `/` (`web/player.html`)

1. Name field + Join. Autofocus. Enter submits.
2. After join: one full-viewport buzzer. `touch-action: manipulation`; `user-select: none`; viewport `width=device-width, maximum-scale=1, user-scalable=no`.
3. Listen to `pointerdown` on the buzzer (keyboard: Space / Enter when focused).
4. While not accepting: buzzer disabled, label “Wait”.
5. After this player’s first accepted buzz: show place (`1st` / `2nd` / `3rd` / `Nth`). Do not show other names.
6. On arm: return to armed buzzer (place cleared).
7. Reconnect: store `playerId` in `sessionStorage`, send it on hello.

### Board `/board` (`web/board.html`)

1. If `k` query missing or `hello` fails: “Open this board from the Buz It app.”
2. Top: accepting state (`ARMED` / `LOCKED`) and player count.
3. Main: if sequence empty and armed, “BUZZ”. If sequence non-empty, **#1 name huge**, then a ranked list.
4. First time `sequence.length` becomes 1 for this `roundId`, play `web/public/lockout.wav`. Mute button; persist mute in `localStorage`.
5. Footer: player join QR + URL(s); smaller clicker QR labeled “Host clicker”.
6. Designed for 1080p TV. Names wrap; list scrolls if > ~12.

### Clicker `/host` (`web/host.html`)

1. Requires `k`. Two huge buttons: **ARM** and **RESET**.
2. Status line: armed/locked, connected player count, current #1 name if any.
3. No buzzer. Host walking the room should not need the TV for arm/reset.

## Audio

- Asset: short lockout sting, ~200–400 ms, `web/public/lockout.wav`.
- Board only. Play via `Audio` element loaded at board hello (so the first buzz is not a download).
- Autoplay: board is loopback/Tauri; call `audio.load()` on first pointer on the mute button as a fallback unlock if the OS blocks it.
- Mute default: unmuted.

## Frontend stack

Vite + TypeScript, **no React**. Three HTML entries. Pages are event-driven DOM updates from snapshots. The buzzer page must stay small so cheap phones parse it quickly.

Dev: `vite build --watch` into `web/dist`; Axum serves `web/dist` with `tower-http` `ServeDir`. One port for UI and WS. No split vite-dev-server for v1.

WS URL: `ws://${location.hostname}:${location.port}/ws` so the board uses loopback and phones use the LAN host they loaded.

## Tauri 2 host

- Window: 1280×720, resizable, title “Buz It”.
- On setup: generate host key, bind server, `window.navigate(http://127.0.0.1:{port}/board?k={key})`.
- Capabilities: allow the webview to load `http://127.0.0.1:*` and connect `ws://127.0.0.1:*`. Do not allow arbitrary remote URLs.
- `beforeDevCommand` / `beforeBuildCommand`: build the Vite app.
- macOS: if a local-network usage string is required for the chosen OS version, set `NSLocalNetworkUsageDescription` to “Phones on your Wi‑Fi connect to this quiz buzzer.”

Headless (optional, not v1 UI): `buz-it-core` can expose a `run` binary later. v1 ships only the Tauri app.

## Testing

**Must have (automated):**

- `Room` unit tests: arm/reset, first buzz wins, sequence order, duplicate buzz ignored, buzz while idle ignored, unknown player, bad host key, name validation, reconnect `connected` flag, room full, arm clears sequence.
- Actor tests: two buzz commands enqueued, snapshot has stable order matching send order.

**Should have:**

- Axum test client: hello as player, hello as clicker with bad key, arm then buzz, snapshot JSON shape.

**Manual (before calling v1 done):**

- Laptop + two phones on the same Wi‑Fi.
- Arm, both tap, board order matches perceived tap, first sound once, phones show 1st/2nd.
- Reset keeps names; arm clears.
- Kill Wi‑Fi on one phone, it shows disconnected on board; reconnect resumes the same id.
- Guest-network failure mode documented if tried.

## Constraints and risks

| Risk | Mitigation |
|---|---|
| Client isolation | Board copy + README |
| Wrong NIC (VPN IP in QR) | Show every private IPv4 |
| `click` 300 ms delay | `pointerdown` only |
| Double-tap zoom | viewport + `touch-action` |
| Sound blocked | loopback board + mute unlock |
| Host key in screenshots of the board | acceptable on a private LAN |
| 200 sockets | trivial for Tokio; test with ~20 real devices |

## Key decisions

1. **LAN-only Tauri + Axum, not Cloudflare** — product is in-room; cloud adds RTT and an account we do not want.
2. **Rust server + web UI, not all-Rust WASM** — phones are browsers; WASM buys nothing on fairness and costs tap reliability.
3. **Tauri in v1, not a later wrap** — the quizmaster UX *is* the product.
4. **One actor owns `Room`** — total order for buzzes without explicit locks.
5. **Server arrival time** — only honest ranking on untrusted clients.
6. **In-memory state** — a quiz night is ephemeral; skip SQLite.
7. **Three HTML pages, no React** — less machinery on the buzzer.
8. **Board and phones share one protocol** — no Tauri IPC duplicate path.
9. **Arm clears, reset freezes** — matches lockout + sequence without a third “next” button.
10. **No room code** — the URL is the join key while one process = one room.

## Implementation plan (summary)

Detail lives in `docs/superpowers/plans/2026-09-13-buz-it.md`. Phases:

1. Workspace + `Room` TDD (protocol-correct lockout).
2. Actor + Axum `/ws` + static files + LAN IP helper.
3. Web pages: player, board (sound + QR), clicker.
4. Tauri shell opens the board and binds `0.0.0.0`.
5. README operator notes + manual LAN verification.

Do not implement cloud, scoring, or a second transport.

## Open questions (resolved here)

| Question | Resolution |
|---|---|
| Short join code? | No. QR + URL. |
| Kick players? | No in v1. |
| Headless binary? | Not in v1. |
| HTTPS for phone haptics? | No. Optional vibrate if available. |
| Default port clash? | `7423`, overridable. |
