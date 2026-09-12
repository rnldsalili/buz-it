# Buz It artwork

Original geometric buzzer artwork created for the approved tabletop-game redesign.

- `logo.svg`: full-color mark and outlined Barlow Condensed wordmark.
- `logo-mono.svg`: single-color logo; inherits `currentColor` when inlined.
- `mark.svg`: standalone vermilion mark.
- `mark-mono.svg`: standalone single-color mark.
- `../favicon.svg`: ivory mark on a vermilion rounded square; source for Tauri icons.

SVG wordmarks contain outlined paths and require no installed fonts. Inline UI marks use the same geometry with theme-aware `currentColor`. Font licensing is in `../fonts/`.

The PNG app icons in `src-tauri/icons` are browser-rendered from the favicon SVG at high resolution and downsampled to 512, 128, and 32 pixels. Their embedded provenance records the vector source. Use a browser renderer that supports SVG `currentColor` when regenerating them.

Tauri requires PNG color type 6 (RGBA), including the 32px icon. When exporting with ImageMagick, use `-define png:color-type=6` to prevent small icons from becoming indexed-color PNGs. Verify regenerated icons with `npx tauri dev`, since the frontend build does not validate native app icons.
