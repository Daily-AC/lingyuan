#!/usr/bin/env python3
"""tile-only 后处理：跳过去背景，直接降到 32x32 + 量化 5 色 + 全 opaque。"""
import sys
from pathlib import Path
from PIL import Image

PALETTE_RGB = [
    (0x5C, 0x8C, 0x6A),
    (0xD9, 0xA4, 0x41),
    (0x2A, 0x28, 0x26),
    (0xB8, 0x3A, 0x2E),
    (0xF2, 0xEF, 0xE4),
]

SRC = Path(__file__).parent.parent / "assets" / "sprites_raw_v2" / "tile"
DST = Path(__file__).parent.parent / "assets" / "sprites" / "tile"
DST.mkdir(parents=True, exist_ok=True)


def process(src_path: Path, dst_path: Path, size: int = 32) -> None:
    img = Image.open(src_path).convert("RGB")
    # LANCZOS 到 4× 再 NEAREST 到目标，减少摩尔条纹
    inter = img.resize((size * 4, size * 4), Image.Resampling.LANCZOS)
    small = inter.resize((size, size), Image.Resampling.NEAREST)
    # 量化
    pal = Image.new("P", (1, 1))
    flat: list[int] = []
    for r, g, b in PALETTE_RGB:
        flat.extend([r, g, b])
    flat.extend([0] * (768 - len(flat)))
    pal.putpalette(flat)
    quantized = small.quantize(palette=pal, dither=Image.Dither.NONE).convert("RGBA")
    # 全 opaque
    px = quantized.load()
    for y in range(size):
        for x in range(size):
            r, g, b, _ = px[x, y]
            px[x, y] = (r, g, b, 255)
    quantized.save(dst_path, format="PNG")


def main() -> int:
    if not SRC.exists():
        print(f"src 不存在: {SRC}", file=sys.stderr)
        return 2
    n = 0
    for p in SRC.glob("*.png"):
        out = DST / p.name
        process(p, out)
        print(f"  {p.name} -> {out}")
        n += 1
    print(f"完成 {n} 张")
    return 0


if __name__ == "__main__":
    sys.exit(main())
