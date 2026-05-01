---
tags:
  - forgexalith
  - branding
created: 2026-04-15
---

# _branding — Forgexalith

Brand assets of the Forgexalith project : logo sources, mood references,
exports for publication. Everything needed to regenerate app icons,
splash screens, GitHub banners, and other visual material from a single
source of truth.

## Structure

```
_branding/
  README.md              # this file
  source/                # master files (hi-res originals, SVG, PSD)
  mood/                  # moodboards, creative references, iterations
  exports/               # generated derivatives (PNG multi-size, ICO, etc.)
                         # usually gitignored if large
```

## Convention

| Sub-folder | Content | Gitignored ? |
|---|---|---|
| `source/` | Master hi-res (1024+ px PNG/JPG, editable SVG, PSD) | **No** — version-controlled |
| `mood/` | Moodboards, Canva/ComfyUI/DALL-E iteration screenshots | Optional (can be heavy) |
| `exports/` | Generated PNG multi-size, .ico, .icns, banners, favicons | **Yes** (script-generated) |

Note : `exports/` is the output of the `scripts/prepare_icons.py` pipeline.
It is regenerated on demand, so it does not need to be versioned.

## Logo workflow

1. Drop the master in `source/` (e.g. `source/logo-flat-source.jpg`).
2. Run the pipeline :
   ```powershell
   cd <Forgexalith root>
   python scripts/prepare_icons.py _branding/source/logo-flat-source.jpg
   ```
3. The icons are written directly into `forge-pkm/src-tauri/icons/`.
4. `cargo tauri build` automatically picks up the new icons.

## Current files

### source/
_(To be completed — drop the high-resolution original here.)_

### mood/
_(To be completed — save Canva iterations here for creative reference.)_

### exports/
_(Generated automatically by the scripts.)_

## Iteration history

| Date | Iter | Concept | Decision |
|---|---|---|---|
| 2026-04-15 | A-H | 8 candidates, 2 styles (gradient + pixel) | Rejected — too generic |
| 2026-04-15 | I-L | Shield + hammers + obsidian | Inspiring but too busy |
| 2026-04-15 | M-P | Minimal prehistoric arrowhead | Too thin |
| 2026-04-15 | Q-T | Stocky arrowhead in fusion | Better |
| 2026-04-15 | U-X | Flint mix + backdrop shield | Step back |
| 2026-04-15 | Y1-Y4 | Rhizome pattern inside shield | Step back |
| 2026-04-15 | Z1-Z4 | Standalone arrowhead + rhizome tree | Concept validated |
| 2026-04-15 | Moodboard sketch | Crystal + double lava/metal contour + branches | Creative reference |
| 2026-04-15 | **Flat version** (chosen) | Arrowhead + rising orange rhizome, black background, halo | **Working source** |
