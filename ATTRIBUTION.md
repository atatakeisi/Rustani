# Asset attribution

Every non-code asset in this repository (images, icons, fonts, example files, presets) is listed here with its author, source and licence. `cargo xtask assets` (part of `cargo xtask ci`) fails if an asset file is missing from this table.

**Policy (mandatory):** Rustani contains **no Microsoft, Adobe, Avid or Autodesk iconography, images, artwork, fonts, templates, themes, table styles or sample files**. Every asset is original work by GridCraft contributors or third-party material under an open licence (OSI open source, public domain / CC0, or Creative Commons that allows redistribution). Screenshots of Microsoft software are never committed. Font files are not added here: shared fonts live in [storytold/craft-fonts](https://github.com/storytold/craft-fonts), an optional build input (`CRAFT_FONTS_DIR`). The Rustani working tree excludes ArtCraft wordmark and logo images. The upstream brand terms are retained in `docs/brand/LICENSE-brand.txt`; the code licences do not grant rights to ArtCraft marks.

Generated-in-code art is original and has no file to list: the UI icon set (`crates/ui-egui/src/icons.rs`), themes (`crates/engine/src/cmd/view.rs`), cell styles (`crates/engine/src/cmd/format.rs`), table styles (`crates/engine/src/tables.rs`), chart palettes and the sample workbooks (`crates/engine/src/sample.rs`).

## Optional embedded Web font

Wasm builds may embed **BIZ UDPGothic Regular**, by **The BIZ UDGothic Project Authors
(Morisawa Inc.)**, under the [SIL Open Font License 1.1](https://github.com/storytold/craft-fonts/blob/abb83316d96aa59c1cf64784289e378fe9fa5695/fonts/biz-ud-pgothic/OFL.txt).
The external build input is [craft-fonts at `abb83316d96aa59c1cf64784289e378fe9fa5695`](https://github.com/storytold/craft-fonts/tree/abb83316d96aa59c1cf64784289e378fe9fa5695),
with the unmodified font from [morisawa-biz-ud-gothic at `18934af56b9c003ca58c54bffbf226848cb11032`](https://github.com/googlefonts/morisawa-biz-ud-gothic/tree/18934af56b9c003ca58c54bffbf226848cb11032).

The selected file is `fonts/biz-ud-pgothic/BIZUDPGothic-Regular.ttf` **in that external checkout**:
4,669,688 bytes, verified SHA-256 `258d7156c165f2ff774b6efee637c22c3b950de0d8a10e501137061bc8085d01`.
Its source file is not committed to GridCraft. The build reads an existing `CRAFT_FONTS_DIR`
or auto-discovers sibling `../craft-fonts`; no font is embedded when the optional source is absent.
Packages include `OFL-biz-ud-pgothic.txt`, and the Web app displays its embedded OFL at
`?font-license=1`. This font supplies Japanese fallback, with no promise of complete
Chinese/Korean coverage or every Japanese Unicode glyph. See [the integration notes](docs/web-japanese.md).

## Repository assets

| Asset | Author | Source | Licence | Notes |
|---|---|---|---|---|
| `docs/brand/LICENSE-brand.txt` | (licence text) | craftrules | — | |
| `apps/gridcraft-web/pwa/apple-touch-icon.png` | atatakeisi | Resized from `assets/app-icon/rustani-hicolor/512x512/apps/ai.storyteller.gridcraft.png` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | 180px Apple touch icon |
| `apps/gridcraft-web/pwa/icon-192.png` | atatakeisi | Resized from `assets/app-icon/rustani-hicolor/512x512/apps/ai.storyteller.gridcraft.png` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | 192px Web app icon |
| `apps/gridcraft-web/pwa/icon-256.png` | atatakeisi | Copy of `assets/app-icon/rustani-hicolor/256x256/apps/ai.storyteller.gridcraft.png` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Web app icon |
| `apps/gridcraft-web/pwa/icon-512.png` | atatakeisi | Copy of `assets/app-icon/rustani-hicolor/512x512/apps/ai.storyteller.gridcraft.png` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Web app icon |
| `assets/app-icon/rustani-source.png` | atatakeisi (owner-supplied design) | Gemini-generated bird-and-gear JPEG; cleaned with OpenAI image generation; platform exports by `packaging/rustani-icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Rustani app icon; not an ArtCraft trademark |
| `assets/app-icon/rustani-1024.png` | atatakeisi (owner-supplied design) | Gemini-generated bird-and-gear JPEG; cleaned with OpenAI image generation; platform exports by `packaging/rustani-icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Rustani app icon; not an ArtCraft trademark |
| `assets/app-icon/rustani-macos-512.png` | atatakeisi (owner-supplied design) | Gemini-generated bird-and-gear JPEG; cleaned with OpenAI image generation; platform exports by `packaging/rustani-icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Rustani app icon; not an ArtCraft trademark |
| `assets/app-icon/rustani.ico` | atatakeisi (owner-supplied design) | Gemini-generated bird-and-gear JPEG; cleaned with OpenAI image generation; platform exports by `packaging/rustani-icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Rustani app icon; not an ArtCraft trademark |
| `assets/app-icon/rustani.icns` | atatakeisi (owner-supplied design) | Gemini-generated bird-and-gear JPEG; cleaned with OpenAI image generation; platform exports by `packaging/rustani-icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Rustani app icon; not an ArtCraft trademark |
| `assets/app-icon/rustani-hicolor/16x16/apps/ai.storyteller.gridcraft.png` | atatakeisi (owner-supplied design) | Gemini-generated bird-and-gear JPEG; cleaned with OpenAI image generation; platform exports by `packaging/rustani-icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Rustani app icon; not an ArtCraft trademark |
| `assets/app-icon/rustani-hicolor/24x24/apps/ai.storyteller.gridcraft.png` | atatakeisi (owner-supplied design) | Gemini-generated bird-and-gear JPEG; cleaned with OpenAI image generation; platform exports by `packaging/rustani-icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Rustani app icon; not an ArtCraft trademark |
| `assets/app-icon/rustani-hicolor/32x32/apps/ai.storyteller.gridcraft.png` | atatakeisi (owner-supplied design) | Gemini-generated bird-and-gear JPEG; cleaned with OpenAI image generation; platform exports by `packaging/rustani-icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Rustani app icon; not an ArtCraft trademark |
| `assets/app-icon/rustani-hicolor/48x48/apps/ai.storyteller.gridcraft.png` | atatakeisi (owner-supplied design) | Gemini-generated bird-and-gear JPEG; cleaned with OpenAI image generation; platform exports by `packaging/rustani-icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Rustani app icon; not an ArtCraft trademark |
| `assets/app-icon/rustani-hicolor/64x64/apps/ai.storyteller.gridcraft.png` | atatakeisi (owner-supplied design) | Gemini-generated bird-and-gear JPEG; cleaned with OpenAI image generation; platform exports by `packaging/rustani-icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Rustani app icon; not an ArtCraft trademark |
| `assets/app-icon/rustani-hicolor/128x128/apps/ai.storyteller.gridcraft.png` | atatakeisi (owner-supplied design) | Gemini-generated bird-and-gear JPEG; cleaned with OpenAI image generation; platform exports by `packaging/rustani-icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Rustani app icon; not an ArtCraft trademark |
| `assets/app-icon/rustani-hicolor/256x256/apps/ai.storyteller.gridcraft.png` | atatakeisi (owner-supplied design) | Gemini-generated bird-and-gear JPEG; cleaned with OpenAI image generation; platform exports by `packaging/rustani-icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Rustani app icon; not an ArtCraft trademark |
| `assets/app-icon/rustani-hicolor/512x512/apps/ai.storyteller.gridcraft.png` | atatakeisi (owner-supplied design) | Gemini-generated bird-and-gear JPEG; cleaned with OpenAI image generation; platform exports by `packaging/rustani-icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE-rustani.txt`) | Rustani app icon; not an ArtCraft trademark |
| `assets/app-icon/LICENSE-rustani.txt` | atatakeisi | Rustani icon provenance and licence | — | |
| `assets/app-icon/gridcraft.svg` | GridCraft contributors | original (hand-written SVG; `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | PLACEHOLDER app icon (ledger page); replaced by the owner's creature art later |
| `packaging/macos/dmg/background.svg` | @XusBadia | original (hand-written SVG, updated to use the Rustani app icon `assets/app-icon/rustani-1024.png`, referenced via `<image>`, not copied; text is outlined glyph paths; `packaging/macos/dmg/generate.py` renders it to `background.tiff`) | MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`) | GridCraft macOS DMG window background for the Finder window; added 2026-10-08 |
| `packaging/macos/dmg/background.tiff` | @XusBadia | Rendered from `packaging/macos/dmg/background.svg` with resvg 0.49.0, macOS sips and tiffutil | MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`) | 1x/2x LZW TIFF for the macOS DMG window, 72/144 dpi; original layout by @XusBadia, updated Rustani icon by atatakeisi |
| `assets/app-icon/gridcraft-1024.png` | GridCraft contributors | original (hand-written SVG; `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Placeholder icon, rendered from gridcraft.svg |
| `assets/app-icon/gridcraft-macos-512.png` | GridCraft contributors | original (hand-written SVG; `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Placeholder icon, macOS grid render |
| `assets/app-icon/gridcraft.icns` | GridCraft contributors | original (hand-written SVG; `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Placeholder icon, macOS bundle icon |
| `assets/app-icon/gridcraft.ico` | GridCraft contributors | original (hand-written SVG; `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Placeholder icon, Windows exe icon |
| `assets/app-icon/hicolor/16x16/apps/ai.storyteller.gridcraft.png` | GridCraft contributors | original (hand-written SVG; `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Placeholder icon, Linux hicolor render |
| `assets/app-icon/hicolor/24x24/apps/ai.storyteller.gridcraft.png` | GridCraft contributors | original (hand-written SVG; `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Placeholder icon, Linux hicolor render |
| `assets/app-icon/hicolor/32x32/apps/ai.storyteller.gridcraft.png` | GridCraft contributors | original (hand-written SVG; `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Placeholder icon, Linux hicolor render |
| `assets/app-icon/hicolor/48x48/apps/ai.storyteller.gridcraft.png` | GridCraft contributors | original (hand-written SVG; `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Placeholder icon, Linux hicolor render |
| `assets/app-icon/hicolor/64x64/apps/ai.storyteller.gridcraft.png` | GridCraft contributors | original (hand-written SVG; `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Placeholder icon, Linux hicolor render |
| `assets/app-icon/hicolor/128x128/apps/ai.storyteller.gridcraft.png` | GridCraft contributors | original (hand-written SVG; `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Placeholder icon, Linux hicolor render |
| `assets/app-icon/hicolor/256x256/apps/ai.storyteller.gridcraft.png` | GridCraft contributors | original (hand-written SVG; `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Placeholder icon, Linux hicolor render |
| `assets/app-icon/hicolor/512x512/apps/ai.storyteller.gridcraft.png` | GridCraft contributors | original (hand-written SVG; `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Placeholder icon, Linux hicolor render |
| `assets/app-icon/hicolor/scalable/apps/ai.storyteller.gridcraft.svg` | GridCraft contributors | original (hand-written SVG; `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Placeholder icon, copy of gridcraft.svg |
| `assets/app-icon/README.md` | GridCraft contributors; Rustani updates by atatakeisi | original and modified documentation | MIT OR Apache-2.0 | Icon documentation |
| `assets/app-icon/LICENSE.txt` | (licence text) | original | — | Licence of the app icon files |
| `docs/images/hero-sales.png` | GridCraft contributors | screenshot of GridCraft rendered offscreen (`crates/ui-egui/examples/snapshot.rs`) showing an original sample workbook | MIT OR Apache-2.0 | README screenshot; no third-party content |
| `docs/images/japanese-vba.jpg` | GridCraft contributors | Chrome screenshot of the local GridCraft Web app with an original Japanese VBA example and generated worksheet | MIT OR Apache-2.0 | Japanese README screenshot; no Excel screenshot or user workbook data |
| `docs/images/budget.png` | GridCraft contributors | screenshot of GridCraft rendered offscreen (`crates/ui-egui/examples/snapshot.rs`) showing an original sample workbook | MIT OR Apache-2.0 | README screenshot; no third-party content |
| `docs/images/grades-formulas.png` | GridCraft contributors | screenshot of GridCraft rendered offscreen (`crates/ui-egui/examples/snapshot.rs`) showing an original sample workbook | MIT OR Apache-2.0 | README screenshot; no third-party content |
| `docs/images/dark-insert.png` | GridCraft contributors | screenshot of GridCraft rendered offscreen (`crates/ui-egui/examples/snapshot.rs`) showing an original sample workbook | MIT OR Apache-2.0 | README screenshot; no third-party content |
| `docs/images/pivot.png` | GridCraft contributors | screenshot of GridCraft rendered offscreen (`crates/ui-egui/examples/snapshot.rs`) showing an original sample workbook | MIT OR Apache-2.0 | README screenshot; no third-party content |
| `docs/images/format-chart.png` | GridCraft contributors | screenshot of GridCraft rendered offscreen (`crates/ui-egui/examples/snapshot.rs`) showing an original sample workbook | MIT OR Apache-2.0 | README screenshot; no third-party content |
| `docs/images/page-break.png` | GridCraft contributors | screenshot of GridCraft rendered offscreen (`crates/ui-egui/examples/snapshot.rs`) showing an original sample workbook | MIT OR Apache-2.0 | README screenshot; no third-party content |

## Bundled dependency and font notices

[THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt) contains the notices collected for the Web application's normal runtime dependencies and its five bundled font families. They keep their own licences; the application licence does not relicense them. The collection documents its scope and exclusions.
