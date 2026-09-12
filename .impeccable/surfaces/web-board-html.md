---
version: 1
slug: "web-board-html"
primary_target: "web/board.html"
related_targets: ["web/player.html"]
---

# Board and player redesign
Mode: Operate. Targets: board and player routes, all game states. Approved code-first workflow.

## Direction contract
THESIS: A social quiz lives on a tabletop: bold game-box lettering and numbered pieces make the first buzz unmistakable.
OWN-WORLD: Ivory paper, dark ink, vermilion, Barlow Condensed display and DM Sans controls; flat surfaces, deliberate borders, a custom buzzer silhouette. Light defaults for shared living-room use; dark is available for dim rooms.
STORY: Scan a QR, enter a name, wait for the host, tap immediately, see your place. The host starts or closes a round without obscuring results.
FIRST VIEWPORT: Board: logo and connection above, oversized winner left, QR joining right, ordered list below, host console at the bottom. Phone: short name form then a large circular buzzer, identity above, connection below.
FORM: User-pinned tabletop game direction overrides seed a1b8471c (assigned index 4); no additional concept selection needed. Signature interaction is the first-place token reveal.
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

No unresolved product decisions. Preserve every LAN address, accessibility, reconnect gating, and existing game semantics. No protocol changes.
