# Buz It

<!-- impeccable:product-schema 1 -->

## Platform
web

## Users
Friends, families, and community groups playing social quiz nights. A host operates a laptop connected to a TV; players use phone browsers.

## Product Purpose
Make it easy to join a local quiz and see who buzzed first, followed by the complete arrival order.

## Operating Context
One running app is one room. Everyone uses the same local Wi-Fi. The laptop runs a Tauri app or standalone server; this is not remote or cloud play.

## Capabilities and Constraints
Name entry, QR joining for every detected LAN address, start round, close buzzing, full ordered results, reconnect feedback, sound mute, and per-browser light/dark themes. Starting clears results; closing preserves them. Host control requires loopback and the host key. Buzzes fire on pointer down, require authenticated fresh state, and are never queued offline. Preserve the current protocol and session identity.

## Brand Commitments
Keep the name Buz It. The user approved a tabletop-game identity: warm ivory, dark ink, vermilion, bold lettering, numbered results, and an original buzzer emblem. Rework layouts and flows without adding game capabilities.

## Product Principles
- First-time players can join without instruction from the host.
- The first player and full buzz order are easy to read on a TV.
- Connection and round state are explicit.
- No internet resources are needed during play.

## Accessibility & Inclusion
Keyboard and assistive activation, visible focus, text-based state feedback, reduced motion, and WCAG AA text contrast. Layouts support narrow and landscape phones.
