#!/usr/bin/env python3
"""校验 assets/sprites/ 是否齐全 + 是否符合 32x32 + 5 色调色板。"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from gen_sprites import MANIFEST, OUT_DIR, PALETTE_RGB  # noqa: E402


def main() -> int:
    rc = 0
    print(f"扫 {OUT_DIR}")
    missing = []
    for category, name, _ in MANIFEST:
        p = OUT_DIR / category / f"{name}.png"
        if not p.exists():
            missing.append(p)
    if missing:
        print(f"❌ 缺 {len(missing)} 个 sprite:")
        for p in missing[:20]:
            print(f"  {p}")
        rc = 1
    else:
        print(f"✅ {len(MANIFEST)} 个 sprite 齐全")

    # 调色板合规校验
    try:
        from PIL import Image
    except ImportError:
        print("⚠️  pillow 没装，跳过调色板校验")
        return rc

    pal_set = set(PALETTE_RGB) | {(0, 0, 0)}  # 0,0,0 是透明像素 fallback
    bad = []
    for p in OUT_DIR.rglob("*.png"):
        img = Image.open(p).convert("RGBA")
        if img.size != (32, 32):
            bad.append((p, f"size={img.size}"))
            continue
        for x in range(32):
            for y in range(32):
                r, g, b, a = img.getpixel((x, y))
                if a == 0:
                    continue
                if (r, g, b) not in pal_set:
                    bad.append((p, f"pixel ({x},{y}) = {(r, g, b)} 不在调色板"))
                    break
            else:
                continue
            break
    if bad:
        print(f"⚠️  {len(bad)} 个 sprite 不合规（前 10 个）:")
        for p, reason in bad[:10]:
            print(f"  {p}: {reason}")
        rc = 1
    else:
        print("✅ 所有 sprite 符合 32x32 + 5 色调色板")
    return rc


if __name__ == "__main__":
    sys.exit(main())
