#!/usr/bin/env python3
"""Regenerate checked-in Nova icons; authoring only, requires rsvg-convert."""

import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "packaging/linux/icons/hicolor"


def main() -> None:
    renderer = shutil.which("rsvg-convert")
    if renderer is None:
        raise SystemExit("Install librsvg2-bin (Debian/Ubuntu) or librsvg (Arch) to render icons.")
    source = (ICONS / "scalable/apps/nova-control.svg").read_text(encoding="utf-8")
    for size in (16, 22, 24, 32, 48, 64, 128, 256, 512):
        # Optical weights keep the fine-line mark legible at panel sizes.
        weight = 14 if size <= 16 else 11 if size <= 24 else 9 if size <= 32 else 7
        svg = source.replace('stroke-width="7"', f'stroke-width="{weight}"')
        if size <= 24:
            svg = svg.replace('141 28V40', '141 28').replace('115 228V216', '115 228')
        target = ICONS / f"{size}x{size}/apps/nova-control.png"
        target.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run([renderer, "-w", str(size), "-h", str(size), "-o", str(target)],
                       input=svg.encode("utf-8"), check=True)
        print(target.relative_to(ROOT))


if __name__ == "__main__":
    main()
