#!/usr/bin/env python3
"""
灵渊 sprite 生成脚本（M8）

跑前置：
    pip install openai pillow
    export OPENAI_API_KEY=sk-...

跑：
    python scripts/gen_sprites.py [--only tile|building|creature|plant|agent|item]

输出：
    assets/sprites/{category}/{name}.png （32x32 透明背景，已 quantize 到 5 色调色板）

预算估算：
    ~60 张 × gpt-image-1 ~$0.04 = < $3
"""
from __future__ import annotations

import argparse
import io
import os
import sys
from pathlib import Path
from typing import Iterable

# 5 色调色板（与 spec 一致）
PALETTE_RGB = [
    (0x5C, 0x8C, 0x6A),  # 青竹绿
    (0xD9, 0xA4, 0x41),  # 落日金
    (0x2A, 0x28, 0x26),  # 玄墨黑
    (0xB8, 0x3A, 0x2E),  # 朱砂红
    (0xF2, 0xEF, 0xE4),  # 月白
]

BASE_PROMPT = (
    "Pixel art sprite, 32x32 resolution, top-down view, xianxia (Chinese fantasy / wuxia) aesthetic, "
    "limited 5-color palette (jade green #5C8C6A, sunset gold #D9A441, ink black #2A2826, "
    "cinnabar red #B83A2E, moon white #F2EFE4). Transparent background. Clean pixel edges, "
    "no anti-aliasing. {EXTRAS}"
)

# (category, name, extras)
MANIFEST: list[tuple[str, str, str]] = [
    # ─── tile (13) ───────────────────────────────────
    ("tile", "grass", "Lush grass texture, soft green tufts."),
    ("tile", "bamboo_forest", "Dense bamboo grove top-down, shadows of stalks."),
    ("tile", "pine_forest", "Pine canopy top-down, dark green needles."),
    ("tile", "reed", "Tall riverside reeds with wind sway."),
    ("tile", "maple", "Red maple canopy top-down, cinnabar leaves."),
    ("tile", "sand", "Loose sand, ripples and tiny pebbles."),
    ("tile", "stone", "Cracked grey stone slabs."),
    ("tile", "mountain", "Sharp mountain rock face from above, snow caps."),
    ("tile", "shallow_water", "Shallow flowing water with stones beneath."),
    ("tile", "deep_water", "Deep dark water surface, faint ripples."),
    ("tile", "ruin", "Cracked ancient stone temple ruins."),
    ("tile", "road", "Worn dirt path with cart tracks."),
    ("tile", "ash", "Burned ground with grey ash."),

    # ─── building (2) ────────────────────────────────
    ("building", "campfire", "Pile of burning logs with rising orange flame, top-down isometric."),
    ("building", "cooking_stove", "Stone cooking stove with iron pot, side-front pixel view."),

    # ─── creature (4 × 2 朝向 = 8) ────────────────────
    ("creature", "rabbit_left", "Cute white rabbit facing left, top-down, ears up."),
    ("creature", "rabbit_right", "Cute white rabbit facing right, top-down, ears up."),
    ("creature", "deer_left", "Brown deer with small antlers facing left, top-down."),
    ("creature", "deer_right", "Brown deer with small antlers facing right, top-down."),
    ("creature", "wolf_left", "Grey wolf snarling facing left, top-down, fierce."),
    ("creature", "wolf_right", "Grey wolf snarling facing right, top-down, fierce."),
    ("creature", "night_demon_left", "Dark purple-black demon spirit floating, two glowing red eyes, facing left."),
    ("creature", "night_demon_right", "Dark purple-black demon spirit floating, two glowing red eyes, facing right."),

    # ─── plant (10) ───────────────────────────────────
    ("plant", "bamboo_stalk", "Single fresh bamboo stalk with leaves, top-down."),
    ("plant", "pine_log", "Felled pine log, dark brown with rings on cut end."),
    ("plant", "stone_chunk", "Pile of small grey stones."),
    ("plant", "flint_chunk", "Sharp golden flint stone shard."),
    ("plant", "clay_lump", "Wet brown clay lump."),
    ("plant", "lingzhi", "Red ganoderma (lingzhi) mushroom with glossy cap, mystical."),
    ("plant", "mushroom", "Small forest mushroom, pink cap with white spots."),
    ("plant", "red_berry", "Cluster of red berries on green vine."),
    ("plant", "vine", "Coiled green vine on the ground."),
    ("plant", "reed", "Bundle of tall reeds tied with twine."),

    # ─── agent (4 朝向) ────────────────────────────────
    ("agent", "default_north", "Xianxia cultivator in moon-white robe facing north (back), top-down, neutral pose."),
    ("agent", "default_south", "Xianxia cultivator in moon-white robe facing south (front), top-down."),
    ("agent", "default_east", "Xianxia cultivator in moon-white robe facing east (right), top-down."),
    ("agent", "default_west", "Xianxia cultivator in moon-white robe facing west (left), top-down."),

    # ─── item icons (~20) — 24x24 显示 ────────────────
    ("item", "bamboo", "Single bamboo stalk icon."),
    ("item", "pinewood", "Stack of pinewood planks icon."),
    ("item", "stone", "Small grey stone icon."),
    ("item", "flint", "Golden flint shard icon."),
    ("item", "clay", "Brown clay ball icon."),
    ("item", "vine", "Coiled green vine icon."),
    ("item", "reed", "Bundle of reeds icon."),
    ("item", "lingzhi", "Red lingzhi mushroom icon, glossy."),
    ("item", "mushroom", "Pink-cap mushroom icon."),
    ("item", "red_berry", "Cluster of red berries icon."),
    ("item", "bamboo_spear", "Bamboo spear with sharpened flint tip icon."),
    ("item", "stone_axe", "Stone axe with wooden handle icon."),
    ("item", "rope", "Coiled hemp rope icon."),
    ("item", "clay_pot", "Brown clay pot icon."),
    ("item", "cooked_mushroom", "Skewer of grilled mushrooms icon."),
    ("item", "cooked_berry", "Roasted red berries icon."),
    ("item", "rice_cake", "Steamed white rice cake icon."),
    ("item", "campfire_kit", "Bundle of logs + flint, campfire kit icon."),
    ("item", "cooking_stove_kit", "Stack of stones + clay, cooking stove kit icon."),
]

OUT_DIR = Path(__file__).parent.parent / "assets" / "sprites"


def call_openai(prompt: str, size: str = "1024x1024") -> bytes:
    """调 OpenAI gpt-image-1，返回 PNG bytes"""
    from openai import OpenAI  # 延迟导入避免无 key 也能 import 本脚本

    client = OpenAI()
    resp = client.images.generate(
        model="gpt-image-1",
        prompt=prompt,
        size=size,
        n=1,
    )
    import base64

    b64 = resp.data[0].b64_json
    if b64 is None:
        raise RuntimeError(f"OpenAI 返回无 b64_json: {resp}")
    return base64.b64decode(b64)


def quantize_to_palette(img_bytes: bytes, target: int = 32) -> bytes:
    """缩到 32x32、量化到 5 色调色板、保留 alpha"""
    from PIL import Image

    img = Image.open(io.BytesIO(img_bytes)).convert("RGBA")
    img = img.resize((target, target), Image.Resampling.NEAREST)

    # 提 alpha
    rgb = img.convert("RGB")
    alpha = img.split()[3]

    # 构造调色板 PIL Image
    pal = Image.new("P", (1, 1))
    flat = []
    for r, g, b in PALETTE_RGB:
        flat.extend([r, g, b])
    flat.extend([0] * (768 - len(flat)))
    pal.putpalette(flat)
    quantized = rgb.quantize(palette=pal, dither=Image.Dithering.NONE)
    quantized = quantized.convert("RGBA")

    # 还原 alpha；alpha < 128 设 0，>= 128 设 255（硬边）
    px = quantized.load()
    ap = alpha.load()
    for y in range(target):
        for x in range(target):
            a = ap[x, y]
            r, g, b, _ = px[x, y]
            px[x, y] = (r, g, b, 0 if a < 128 else 255)

    out = io.BytesIO()
    quantized.save(out, format="PNG")
    return out.getvalue()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--only",
        type=str,
        default=None,
        help="只生成指定 category（tile/building/creature/plant/agent/item）",
    )
    ap.add_argument("--dry-run", action="store_true", help="只列要做啥，不调 API")
    ap.add_argument("--skip-existing", action="store_true", help="若文件已存在则跳过")
    args = ap.parse_args()

    if not args.dry_run and "OPENAI_API_KEY" not in os.environ:
        print("错误：没设 OPENAI_API_KEY。export 一下再跑。")
        sys.exit(2)

    items: Iterable[tuple[str, str, str]] = MANIFEST
    if args.only:
        items = [t for t in MANIFEST if t[0] == args.only]
        if not items:
            print(f"错误：category '{args.only}' 没匹配")
            sys.exit(2)

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    total = len(list(items))
    done = 0
    for category, name, extras in items:
        out_path = OUT_DIR / category / f"{name}.png"
        out_path.parent.mkdir(parents=True, exist_ok=True)
        if args.skip_existing and out_path.exists():
            print(f"[{done+1}/{total}] skip {category}/{name} (exists)")
            done += 1
            continue
        prompt = BASE_PROMPT.format(EXTRAS=extras)
        print(f"[{done+1}/{total}] {category}/{name}")
        if args.dry_run:
            print(f"    PROMPT: {prompt}")
        else:
            raw = call_openai(prompt)
            quantized = quantize_to_palette(raw, target=32)
            out_path.write_bytes(quantized)
            print(f"    -> {out_path} ({len(quantized)} bytes)")
        done += 1
    print(f"完成 {done}/{total}")


if __name__ == "__main__":
    main()
