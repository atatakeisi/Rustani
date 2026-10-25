# Rustani app icons

The current Rustani artwork is a green Japanese white-eye (mejiro) with a silver gear on a
transparent background. The project owner supplied the Gemini-generated JPEG design, and the
built-in image generator cleaned it into `rustani-source.png`, a transparent 1254 × 1254 PNG.
The supplied ZIP original is retained unchanged. Asset authorship and licence information is
recorded in `ATTRIBUTION.md` at the repository root.

## Current files

| File | Use |
|---|---|
| `rustani-source.png` | Transparent source artwork for the Rustani variants |
| `rustani-1024.png` | 1024 px version, also linked by the macOS DMG background SVG |
| `rustani-macos-512.png` | Runtime Dock icon embedded by `apps/gridcraft` on macOS |
| `rustani.icns` | macOS bundle icon; copied to the existing `GridCraft.icns` bundle resource |
| `rustani.ico` | Web favicon and Windows executable, installer and shortcut icon |
| `rustani-hicolor/<size>/apps/ai.storyteller.gridcraft.png` | Linux/FreeBSD/Flatpak icons at 16, 24, 32, 48, 64, 128, 256 and 512 px; the 256 px version is also the runtime window icon on Windows and Linux |

The internal app ID remains `ai.storyteller.gridcraft`. Packaging installs the contents of
`rustani-hicolor` under the standard `share/icons/hicolor` directory. The Rustani set contains
raster PNG sizes and has no scalable SVG variant.

## Regeneration

Run `bash packaging/rustani-icons.sh` to generate the Rustani PNG, ICO and ICNS variants from
`rustani-source.png`. It uses macOS `sips` and `iconutil` and the existing `cargo xtask ico`
packer, preserving transparency and proportions. Review the small sizes and the macOS margin
before replacing packaged outputs.

`packaging/icons.sh` regenerates the original GridCraft ledger artwork from `gridcraft.svg`.
It does not generate the Rustani icon set and must not be used to update these files.

## Retained original artwork

The original `gridcraft.svg`, `gridcraft-1024.png`, `gridcraft-macos-512.png`, `gridcraft.ico`,
`gridcraft.icns` and `hicolor/` files are retained. They depict a ledger page on a green tile,
original geometry by the GridCraft contributors under MIT OR Apache-2.0 (`LICENSE.txt`).
The old palette is Ink `#0b0b0c`, Paper `#efe9dc` and green `#1f9d55`.

## Artwork processing specification

The built-in image tool was asked to preserve the supplied right-facing green-and-cream bird, white eye-ring, dark beak, feet, warm yellow interior and silver gear; remove photographed-screen moiré; reconstruct cropped gear edges; retain gentle shading; use a square transparent canvas and a small even margin; and add no text, logos or new elements. Native platform exports only resize and encode the selected result.

The Rustani artwork uses `LICENSE-rustani.txt`. The original ledger assets keep `LICENSE.txt`.

<details>
<summary>Built-in image editing prompt (2026-10-25)</summary>

Use case: precise-object-edit. Asset type: Rustani application icon. Edit target: the provided JPEG of a green Japanese white-eye bird (mejiro) framed by a silver gear. Create one clean, square, high-resolution app icon faithfully preserving this exact design: right-facing green-and-cream bird with white eye-ring, black round eye and small white highlight, dark beak, two pink-brown feet grasping the bottom edge, warm pale-yellow inner rounded-square panel, thick dark outline, silver toothed gear frame. Keep the composition, colors, character, pose, and relative proportions. Remove the photographed-screen moiré/pixel grid and lighting artifacts; produce smooth crisp vector-like illustrative edges with the same gentle shading. Reconstruct the cropped top and outermost gear edges so the entire symmetrical silver gear fits inside the image with a small, even transparent margin. Outside the gear must be genuine transparency, preserve the pale-yellow interior. No new design elements, no text, no letters, no logos, no watermark. One centered icon only, no mockup, no icon grid. Do not include ArtCraft or GridCraft brand imagery. This is the user's provided design; retain its identity.

The built-in tool used the owner-supplied JPEG as its edit target and requested a transparent background.

</details>
