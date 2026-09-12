---
name: Buz It
description: A tabletop identity for friendly quiz-night competition.
colors:
  ivory: "#f7f0e2"
  paper: "#fffaf0"
  ink: "#252820"
  vermilion: "#b93622"
  soft: "#ebe2d0"
  accent-soft: "#f5e2d3"
  muted: "#686557"
  line: "#c9c1ae"
  line-strong: "#85816f"
  dark-background: "#23261f"
  dark-surface: "#2d3028"
  dark-soft: "#383b31"
  dark-accent: "#ff987c"
  dark-accent-soft: "#50392e"
  dark-muted: "#c1bdad"
  dark-line: "#515447"
  dark-line-strong: "#929580"
typography:
  display:
    fontFamily: "Barlow Condensed, sans-serif"
    fontSize: "clamp(3.5rem, 7.5vw, 6rem)"
    fontWeight: 700
    lineHeight: 1
    letterSpacing: "-0.025em"
  body:
    fontFamily: "DM Sans, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.6
rounded:
  control: "12px"
  tile: "16px"
spacing:
  sm: "8px"
  md: "16px"
  lg: "24px"
  xl: "32px"
components:
  button-primary:
    backgroundColor: "{colors.vermilion}"
    textColor: "{colors.paper}"
    rounded: "{rounded.control}"
    padding: "10px 16px"
  button-secondary:
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
    padding: "10px 16px"
---

# Design System: Buz It

## Overview

**Creative North Star: "The Tabletop Game"**

Warm paper, decisive ink, and a vermilion buzzer make friendly competition feel tangible. Condensed game-box lettering carries personality; simple controls and explicit state messages keep the game easy to operate.

Key characteristics: flat surfaces, bold numbered pieces, generous touch targets, and locally bundled fonts. The user selected this identity and code-first implementation.

## Colors

Light mode defaults to ivory, paper, and ink. Vermilion identifies the main action and first place; soft paper separates the join area. Dark mode uses dark-background, dark-surface, ivory text, and dark-accent. The authoritative theme mappings are in `web/src/shared.css`; use semantic CSS variables rather than hardcoded component colors.

Status always includes text. The accent is never the only way to distinguish an open round, first place, or a connection problem.

## Typography

Barlow Condensed Bold is for the wordmark, headings, ranks, and buzzer label. DM Sans is for controls and supporting text. Both ship locally with their OFL licenses. Wordmark exports contain outlined paths. Display type caps at 6rem; long winner names use a smaller responsive size and wrap without truncating.

## Layout

At 1000px and above, the board fills the viewport with an 88px header, flexible results area, and host console at the bottom. The QR column is 264px, growing to 320px at 1600px. Results and join codes scroll independently. Under 1000px the join area stacks below results. Below 520px the winner token stacks above the name.

Phone content is centered in a 400px column with safe-area padding. Identity stays above the buzzer; connection is below. Short landscape phones use a two-column playing layout with a 210px buzzer. The full buzz order remains scrollable and keyboard focusable.

## Elevation & Depth

Flat fills and deliberate borders define hierarchy. There are no decorative gradients or shadows. The active buzzer briefly scales to 0.97 on press; sending the buzz never waits for motion.

## Shapes

Controls use 12px corners, placement tiles 16px, and the phone buzzer is circular. Waiting uses a dashed outline and muted fill; ready uses solid action color. The geometric buzzer logo is authored SVG with theme-aware color in the interface.

## Components

Primary controls use action/on-action colors; secondary controls use transparent fill and a visible border. Controls have at least 44px height, with 56px phone form controls. Focus uses a 3px accent outline offset 4px. Disabled controls use muted text and soft surfaces.

First place uses a filled numbered tile and prominent player name. Later placements retain large numbers on a neutral tile. Round state, authentication, and connection messages remain explicit. The player's persistent status region announces state changes only when its text changes.

The winner token has one 360ms reveal using `cubic-bezier(.16,1,.3,1)`. Reduced-motion preferences disable authored animations and transitions. QR images retain their white background in both themes.

## Do's and Don'ts

- Do preserve immediate pointer-down and keyboard activation.
- Do keep every LAN address accessible and full names readable.
- Do use text with color to communicate state.
- Do bundle assets locally for LAN-only operation.
- Don't add decorative gradients, emoji icons, or lightning glyphs.
- Don't hide host controls behind the scrollable result list.
- Don't queue offline taps or imply a connection before a fresh snapshot.
