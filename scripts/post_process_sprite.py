#!/usr/bin/env python3
"""
后处理 gpt-image-2 生成的 sprite：
  1. 去除"checkerboard 透明背景"（实际是画出来的灰白棋盘格）
  2. 降采样到 32×32（保留 nearest 像素感）
  3. 量化到 5 色调色板
  4. 输出带 alpha 的 PNG
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Iterable

from PIL import Image

PALETTE_RGB = [
    (0x5C, 0x8C, 0x6A),  # 青竹绿
    (0xD9, 0xA4, 0x41),  # 落日金
    (0x2A, 0x28, 0x26),  # 玄墨黑
    (0xB8, 0x3A, 0x2E),  # 朱砂红
    (0xF2, 0xEF, 0xE4),  # 月白
]

# 棋盘背景的典型两个灰色（gpt-image-2 风格固定）
CHECKER_LIGHT = (235, 235, 235)
CHECKER_DARK = (200, 200, 200)
CHECKER_TOL = 22  # 容差


def is_checker(rgb: tuple[int, int, int]) -> bool:
    r, g, b = rgb
    if abs(r - g) > 8 or abs(g - b) > 8 or abs(r - b) > 8:
        return False  # 不是灰色
    luma = (r + g + b) / 3
    for cl, cd in [(CHECKER_LIGHT, CHECKER_DARK)]:
        cl_l = sum(cl) / 3
        cd_l = sum(cd) / 3
        if abs(luma - cl_l) <= CHECKER_TOL or abs(luma - cd_l) <= CHECKER_TOL:
            return True
    return False


def strip_checkerboard(src: Image.Image) -> Image.Image:
    """把棋盘 background pixel 设为 alpha=0"""
    rgba = src.convert("RGBA")
    px = rgba.load()
    w, h = rgba.size
    for y in range(h):
        for x in range(w):
            r, g, b, _a = px[x, y]
            if is_checker((r, g, b)):
                px[x, y] = (0, 0, 0, 0)
    return rgba


def flood_fill_corners_transparent(rgba: Image.Image) -> Image.Image:
    """从 4 个角 flood fill 任何接触的"接近棋盘色"的连通块为透明"""
    px = rgba.load()
    w, h = rgba.size
    seen = [[False] * h for _ in range(w)]
    stack: list[tuple[int, int]] = [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)]
    while stack:
        x, y = stack.pop()
        if x < 0 or x >= w or y < 0 or y >= h or seen[x][y]:
            continue
        seen[x][y] = True
        r, g, b, a = px[x, y]
        if a == 0 or is_checker((r, g, b)):
            px[x, y] = (0, 0, 0, 0)
            stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)])
    return rgba


def downscale_to(rgba: Image.Image, target: int) -> Image.Image:
    """Nearest 降采样到 target×target；alpha < 128 视为透明"""
    # 先 LANCZOS 到 4×target 再 NEAREST 到 target，减少棋盘条纹
    intermediate = rgba.resize((target * 4, target * 4), Image.Resampling.LANCZOS)
    small = intermediate.resize((target, target), Image.Resampling.NEAREST)
    px = small.load()
    for y in range(target):
        for x in range(target):
            r, g, b, a = px[x, y]
            px[x, y] = (r, g, b, 0 if a < 128 else 255)
    return small


def quantize_to_palette(small: Image.Image) -> Image.Image:
    """量化到 5 色调色板，保留 alpha"""
    alpha = small.split()[3]
    rgb = small.convert("RGB")

    pal = Image.new("P", (1, 1))
    flat: list[int] = []
    for r, g, b in PALETTE_RGB:
        flat.extend([r, g, b])
    flat.extend([0] * (768 - len(flat)))
    pal.putpalette(flat)
    quantized = rgb.quantize(palette=pal, dither=Image.Dither.NONE).convert("RGBA")

    qpx = quantized.load()
    apx = alpha.load()
    w, h = quantized.size
    for y in range(h):
        for x in range(w):
            r, g, b, _ = qpx[x, y]
            qpx[x, y] = (r, g, b, 0 if apx[x, y] < 128 else 255)
    return quantized


def process_one(src_path: Path, out_path: Path, size: int = 32) -> None:
    img = Image.open(src_path)
    img = strip_checkerboard(img)
    img = flood_fill_corners_transparent(img)
    img = downscale_to(img, size)
    img = quantize_to_palette(img)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    img.save(out_path, format="PNG")


def iter_pairs(roots: Iterable[Path], out_root: Path) -> Iterable[tuple[Path, Path]]:
    for root in roots:
        for src in root.rglob("*.png"):
            rel = src.relative_to(root)  # category/name.png
            out = out_root / rel
            yield src, out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", default="assets/sprites_raw", help="raw 输入目录")
    ap.add_argument("--dst", default="assets/sprites", help="后处理输出目录")
    ap.add_argument("--size", type=int, default=32)
    ap.add_argument("--only", help="只处理某一文件名前缀（不含扩展名）")
    args = ap.parse_args()

    src_root = Path(args.src)
    dst_root = Path(args.dst)
    if not src_root.exists():
        print(f"raw 目录不存在: {src_root}", file=sys.stderr)
        return 2

    pairs = list(iter_pairs([src_root], dst_root))
    if args.only:
        pairs = [(s, d) for s, d in pairs if s.stem == args.only]
    if not pairs:
        print("没要处理的", file=sys.stderr)
        return 1
    for s, d in pairs:
        print(f"  {s} -> {d}")
        process_one(s, d, size=args.size)
    print(f"完成 {len(pairs)} 张")
    return 0


if __name__ == "__main__":
    sys.exit(main())
