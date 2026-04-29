---
tags:
  - forgedsidian
  - scripts
  - build
created: 2026-04-15
---

# Scripts — Forgedsidian

Utility scripts for the Forgedsidian project. Cross-platform Python only
(Windows / macOS / Linux), matching the `cross-platform` principle of the
workspace CLAUDE.md ROOT.

## prepare_icons.py

Generates the full icon set expected by Tauri 2 (Windows + generic) from a
single source image, with automatic background removal and centering.

### Install dependencies (one-time)

```powershell
# From Vault-Pro/Dev/Forgedsidian/
python -m pip install pillow rembg
```

First run of the script will also download the `u2net.onnx` model
(~170 MB) into your user cache. Subsequent runs are fast (2-4 s).

### Usage

```powershell
# From Vault-Pro/Dev/Forgedsidian/ (project root)
python scripts/prepare_icons.py path/to/your/logo.jpg
```

With options:

```powershell
# Skip background removal if the source is already transparent
python scripts/prepare_icons.py path/to/logo.png --no-remove-bg

# Increase padding (more breathing room around the icon)
python scripts/prepare_icons.py path/to/logo.jpg --padding 0.10

# Use a more precise (but slower) background removal model
python scripts/prepare_icons.py path/to/logo.jpg --rembg-model isnet-general-use

# Output to a different directory
python scripts/prepare_icons.py path/to/logo.jpg --output-dir some/other/path
```

### What gets generated

In `forge-pkm/src-tauri/icons/` (default):

| File | Size | Usage |
|------|------|-------|
| `16x16.png` | 16 | Favicon, tiny UI |
| `32x32.png` | 32 | Windows taskbar small |
| `48x48.png` | 48 | Windows shell |
| `64x64.png` | 64 | Windows high-DPI |
| `128x128.png` | 128 | Standard |
| `128x128@2x.png` | 256 | Retina 2x |
| `256x256.png` | 256 | Windows file explorer |
| `icon.png` | 1024 | Generic high-res fallback |
| `icon.ico` | 16/32/48/64/128/256 | Windows bundle (multi-size) |

This matches what `tauri.conf.json > bundle.icon` expects.

### Quality notes

- **Source resolution**: ideally 1024x1024 or larger. Minimum 512x512 for
  acceptable results.
- **Source format**: PNG / JPG / WEBP. Transparency is preserved if
  `--no-remove-bg` is passed.
- **Background removal**: `u2net` (default, fast) handles most logos well.
  If your logo has a subtle halo or soft edges, try
  `--rembg-model isnet-general-use` for higher precision.
- **Padding**: default 5% is tight. Bump to 10-15% if you want more
  breathing room (common for system tray icons).

### After generation

The new icons are picked up automatically by Tauri on the next build:

```powershell
cd forge-pkm
cargo tauri build
```

The `.exe` and `.msi` installers will appear in
`forge-pkm/src-tauri/target/release/bundle/`.

### Troubleshooting

**`ModuleNotFoundError: No module named 'rembg'`**
Run `python -m pip install rembg`.

**`rembg` is slow on first run**
Expected — it downloads the u2net model (~170 MB) once. Subsequent runs
are fast.

**The icon has weird edges after background removal**
The source image may have a complex halo or glow that confuses `u2net`.
Try `--rembg-model isnet-general-use` or pre-process in an image editor
to strip the halo manually, then re-run with `--no-remove-bg`.

**`Pillow` throws `cannot write mode RGBA as ICO`**
Older Pillow versions. Upgrade with `python -m pip install -U pillow`.
