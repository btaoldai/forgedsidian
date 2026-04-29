#!/usr/bin/env python3
"""
Forgedsidian — Icon pipeline

Takes a source logo image and produces all icon sizes + .ico bundle
expected by Tauri, directly into `forge-pkm/src-tauri/icons/`.

Usage (from Vault-Pro/Dev/Forgedsidian/ as working directory):
    python scripts/prepare_icons.py <source_image>
    python scripts/prepare_icons.py <source_image> --no-remove-bg
    python scripts/prepare_icons.py <source_image> --padding 0.10

Dependencies:
    pip install pillow rembg

First run of rembg downloads the u2net.onnx model (~170 MB).
Subsequent runs are fast (CPU ~2-4 s per image).

The script is cross-platform (Windows / macOS / Linux) — no OS-specific
commands, pure Pillow + rembg. Matches the `cross-platform` principle of
the Vault-Pro CLAUDE.md ROOT.
"""

import argparse
import io
import sys
from pathlib import Path

from PIL import Image


# ---------------------------------------------------------------------------
# Icon sizes expected by Tauri 2 (Windows + generic)
# ---------------------------------------------------------------------------
# Each entry: (output filename, pixel size).  The keys match what
# `tauri.conf.json > bundle.icon` references.
SIZES_PNG = [
    ("16x16.png", 16),
    ("32x32.png", 32),
    ("48x48.png", 48),
    ("64x64.png", 64),
    ("128x128.png", 128),
    ("128x128@2x.png", 256),   # Retina: 2x of 128
    ("256x256.png", 256),
    ("icon.png", 1024),        # Generic high-res fallback
]

# ICO bundle combines these sizes into a single Windows icon file.
ICO_SIZES = [16, 32, 48, 64, 128, 256]


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate Forgedsidian app icons from a source logo.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument(
        "source",
        help="Path to source logo image (PNG / JPG / WEBP, any size).",
    )
    parser.add_argument(
        "--output-dir",
        default="forge-pkm/src-tauri/icons",
        help="Output directory (default: forge-pkm/src-tauri/icons).",
    )
    parser.add_argument(
        "--no-remove-bg",
        action="store_true",
        help="Skip background removal (if source is already transparent).",
    )
    parser.add_argument(
        "--padding",
        type=float,
        default=0.05,
        help="Padding around content as fraction of max dim (default: 0.05 = 5%%).",
    )
    parser.add_argument(
        "--rembg-model",
        default="u2net",
        help="rembg model to use (u2net | isnet-general-use | sam). "
             "Default u2net is fast and handles most logos. isnet-general-use "
             "is more precise but slower. Only used when --no-remove-bg is not set.",
    )
    return parser.parse_args()


# ---------------------------------------------------------------------------
# Pipeline steps
# ---------------------------------------------------------------------------
def remove_background(img: Image.Image, model_name: str = "u2net") -> Image.Image:
    """Remove background using rembg; returns RGBA image."""
    try:
        from rembg import remove, new_session
    except ImportError:
        print(
            "ERROR: rembg not installed. Run:\n    pip install rembg",
            file=sys.stderr,
        )
        sys.exit(1)

    session = new_session(model_name)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    buf.seek(0)
    result_bytes = remove(buf.read(), session=session)
    return Image.open(io.BytesIO(result_bytes)).convert("RGBA")


def trim_transparent(img: Image.Image) -> Image.Image:
    """Crop to the bounding box of non-transparent pixels."""
    if img.mode != "RGBA":
        img = img.convert("RGBA")
    bbox = img.getbbox()
    if bbox is None:
        return img
    return img.crop(bbox)


def pad_to_square(img: Image.Image, padding: float = 0.05) -> Image.Image:
    """Center the image in a square canvas with transparent padding."""
    w, h = img.size
    max_dim = max(w, h)
    pad_px = int(max_dim * padding)
    canvas_size = max_dim + 2 * pad_px
    canvas = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
    offset_x = (canvas_size - w) // 2
    offset_y = (canvas_size - h) // 2
    canvas.paste(img, (offset_x, offset_y), img)
    return canvas


def resize_lanczos(img: Image.Image, size: int) -> Image.Image:
    """High-quality downscale."""
    return img.resize((size, size), Image.LANCZOS)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
def main() -> None:
    args = parse_args()

    src = Path(args.source)
    if not src.exists():
        print(f"ERROR: source file not found: {src}", file=sys.stderr)
        sys.exit(1)

    out_dir = Path(args.output_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    print(f"[1/5] Loading source: {src}")
    img = Image.open(src).convert("RGBA")
    print(f"      Input size: {img.size}")

    if not args.no_remove_bg:
        print(f"[2/5] Removing background (model: {args.rembg_model})")
        print("      First run downloads u2net.onnx (~170 MB) — please wait.")
        img = remove_background(img, args.rembg_model)
    else:
        print("[2/5] Skipping background removal (--no-remove-bg)")

    print("[3/5] Trimming transparent borders")
    img = trim_transparent(img)
    print(f"      Content bounding box: {img.size}")

    print(f"[4/5] Centering in square canvas with {args.padding * 100:.0f}% padding")
    master = pad_to_square(img, args.padding)
    print(f"      Master canvas: {master.size}")

    print(f"[5/5] Generating icon set in {out_dir}")
    for filename, size in SIZES_PNG:
        scaled = resize_lanczos(master, size)
        out = out_dir / filename
        scaled.save(out, "PNG", optimize=True)
        print(f"      {filename:20s} ({size}x{size})")

    print(f"      Assembling icon.ico with sizes {ICO_SIZES}")
    ico_imgs = [resize_lanczos(master, s) for s in ICO_SIZES]
    ico_path = out_dir / "icon.ico"
    ico_imgs[0].save(
        ico_path,
        format="ICO",
        sizes=[(s, s) for s in ICO_SIZES],
        append_images=ico_imgs[1:],
    )
    print(f"      {ico_path.name}")

    print()
    print(f"Done. Icons written to: {out_dir.resolve()}")
    print("Next: run `cargo tauri build` from forge-pkm/ to bundle the app.")


if __name__ == "__main__":
    main()
