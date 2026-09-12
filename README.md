# Buz It

Buz It is a local LAN quiz buzzer. The host laptop runs a Tauri app (or the core server); phones on the **same Wi‑Fi** join in the browser and tap to buzz. The TV board shows who was first and the full sequence.

This is **not** a cloud app. Nothing is deployed to Cloudflare or the public internet. It is **not** for remote play — everyone must be on the same local network.

## Download and launch

Download the archive for your computer from the **Build portable apps** workflow's artifacts in GitHub Actions. Extract the artifact download, then extract the ZIP inside it.

- **macOS (Apple Silicon or Intel):** open `Buz-It-macOS-universal.zip`, then double-click `Buz It.app`. You can move it to Applications or another folder.
- **Windows x64:** extract `Buz-It-Windows-x64.zip`, then double-click `Buz It.exe`. It needs the [Microsoft WebView2 Evergreen Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/). If missing, install it once before launching.

The app contains the board, player pages, fonts, sounds, and LAN server. No terminal, project files, Node.js, or Rust is needed. Internet access is not needed for quiz play after any required WebView2 setup. Keep the app running while phones play; quitting it stops the room and server.

These initial builds are unsigned and not notarized. macOS or Windows may show an unknown-developer warning; follow your operating system's approval flow only for a build you trust. Allow local-network access when prompted. Phones still need the same Wi-Fi as the host, as described below.

## Build portable apps (developers)

Install Node.js, Rust, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) on the operating system being built for, then:

```bash
npm ci
npm ci --prefix web
# macOS: install both targets for the universal app
rustup target add aarch64-apple-darwin x86_64-apple-darwin
# Windows: install the x64 target instead
# rustup target add x86_64-pc-windows-msvc
npm run build
```

On macOS this creates `dist/Buz-It-macOS-universal.zip` and the app at `target/universal-apple-darwin/release/bundle/macos/Buz It.app`. On Windows it creates `dist/Buz It.exe` and `dist/Buz-It-Windows-x64.zip`. The UI is rebuilt and embedded automatically. Run each build on its native operating system; the Windows executable uses the installed WebView2 runtime.

For both platforms, manually run **Actions → Build portable apps → Run workflow**. It tests and builds on Mac and Windows runners, then uploads the ZIPs as artifacts. It does not publish a release or sign the apps. The workflow must be present on the repository's default branch to appear in Actions.

## Requirements

Phones must reach the laptop on TCP port **7423** (override with `BUZ_IT_PORT`).

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
npx tauri dev
```

(`npm run dev` does the same. This project uses the npm Tauri CLI, not `cargo tauri`.)

or, after the UI build:

```bash
cargo run -p buz-it-core --bin server
```

Default listen port is **7423**. Override with env `BUZ_IT_PORT`.

## How to operate

- **Board** — the laptop window (put it on the TV). Shows open/closed buzzing status, first buzz, ranked list, and join QR codes.
- **Laptop controls** — use the host console below the results on the laptop. Use **Start round** to clear the previous results and open buzzing. Use **Close buzzing** to stop new buzzes and keep the results.
- **Join codes** — the board sidebar starts expanded. Use **Hide join codes** for more result space and **Show join codes** for late arrivals. All detected LAN addresses remain available.
- **Connection** — phones show reconnecting feedback and disable game controls until a fresh connection and round snapshot arrive. Offline taps are never queued.
- **Theme** — light is the default. Use the Light/Dark switch in the board or player header; the choice is saved in that browser and does not change other devices.
- **Players** — scan the player QR, type a name, then tap the buzzer (fires on pointer down).
- **Refresh** — the same phone tab remembers the player name and ID for its browser session. Refreshing automatically rejoins and restores the current round placement after the server confirms it. If browser storage is blocked, playing still works but refresh requires joining again.

Host controls require the current host key and a loopback connection from the laptop. Phone host links and the old clicker role are no longer supported. Use the printed `127.0.0.1` board URL when running the standalone server. Restart the server after upgrading to disconnect any old clicker sessions.

Ranking is server arrival order on the host process. One running app = one room.

## Verify

Automated lockout order (no phones required):

```bash
cargo test -p buz-it-core
```

After `npm run build:ui`, run `cargo test --workspace --locked` to also verify the desktop app’s embedded page/asset responses, content types, HEAD requests, and 404 behavior.

Frontend verification:

```bash
npm run typecheck --prefix web
npm run build:ui
# First-time browser setup:
cd web
npx playwright install chromium
npm run test:ui
```

Browser regressions cover round states, hidden panels, authentication/reconnect gating, inline errors, keyboard input, mute, joining-panel collapse, saved themes, and responsive layouts in both light and dark mode. They use a controlled WebSocket transport for deterministic edge cases.

Quiz-night two-phone checklist — still do this on real devices before the event. This repo does **not** claim those steps were executed here.

1. Launch Tauri. Board shows URL + QR.
2. Phone A and B join with names.
3. On the laptop board, select **Start round**.
4. Both tap. Board: first name huge, list ordered, sound once.
5. Phones show `1st` / `2nd`.
6. **Close buzzing**: unplaced players see Wait; placements and the list remain.
7. **Start round**: list clears; phones back to BUZZ.
8. Toggle mute; first buzz silent then unmuted.
9. Airplane mode phone A: board `connected: false`; reconnect: same name/id.
10. Confirm `click` is not required (tap on down).

Same SSID as the host, not guest Wi‑Fi, client isolation off.

## Visual identity

The tabletop identity uses warm ivory, dark ink, and vermilion, with locally bundled Barlow Condensed and DM Sans fonts. The host console sits below the audience results; phones move from a short name form to a large circular buzzer and a numbered placement tile. Light and dark modes use the same state hierarchy.

The source logo, monochrome variants, and standalone mark live in `web/public/brand/`; the favicon is `web/public/favicon.svg`. The wordmark SVGs contain outlined lettering and need no font installation. Tauri icons are rendered from the favicon artwork. Font license files are bundled in `web/public/fonts/`.

Product context is in [PRODUCT.md](PRODUCT.md); the implemented tokens and component guidance are in [DESIGN.md](DESIGN.md).

## Docs

| Document | Path |
|---|---|
| Design (product, protocol, architecture) | [`docs/superpowers/specs/2026-09-13-buz-it-design.md`](docs/superpowers/specs/2026-09-13-buz-it-design.md) |
| Implementation plan (task-by-task) | [`docs/superpowers/plans/2026-09-13-buz-it.md`](docs/superpowers/plans/2026-09-13-buz-it.md) |

## Stack

Rust (`buz-it-core` + Axum WebSocket) + Vite/TypeScript pages, wrapped in **Tauri 2** so the quizmaster can run a window instead of a terminal.
