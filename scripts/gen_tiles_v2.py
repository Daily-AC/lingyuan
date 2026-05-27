#!/usr/bin/env python3
"""tile sprite v2：用严格 prompt 生成可平铺地砖纹理。"""
import concurrent.futures
import subprocess
import sys
import time
from pathlib import Path

GENERATE = Path.home() / ".claude" / "skills" / "gpt-image-2" / "tools" / "generate.py"
OUT_DIR = Path(__file__).parent.parent / "assets" / "sprites_raw_v2" / "tile"
OUT_DIR.mkdir(parents=True, exist_ok=True)

PROMPT_PREFIX = (
    "SINGLE seamless 32x32 ground texture tile, edge-to-edge fill of the entire square frame, "
    "pixel art. ABSOLUTELY NO foreground objects, NO characters, NO animals, NO buildings, "
    "NO labels, NO decorations, NO frames, NO grid lines. Just bare repeating ground surface. "
    "Limited 5-color palette: jade green #5C8C6A, sunset gold #D9A441, ink black #2A2826, "
    "cinnabar red #B83A2E, moon white #F2EFE4. "
    "Result must look like a video game ground tile, NOT artwork."
)

TILES = {
    "bamboo_forest": "Dense bamboo grove canopy seen from directly above, vertical bamboo segments in jade green, scattered fallen leaves.",
    "pine_forest": "Pine forest canopy from directly above, dark green needle texture, scattered pine cone shapes.",
    "reed": "Marsh reeds and grass seen from above, vertical pale green strokes, occasional water hint.",
    "maple": "Maple forest from above, cinnabar red leaves with dark trunks peeking through.",
    "sand": "Bare sandy ground, sunset gold dunes, tiny dark pebbles scattered, ripple pattern.",
    "stone": "Cracked grey stone pavement, irregular slabs separated by ink-black cracks.",
    "mountain": "Sharp rocky mountain surface from above, dark grey with white snow caps in cracks.",
    "shallow_water": "Shallow flowing water from above, light blue with pebbles visible beneath, gentle ripples.",
    "deep_water": "Deep dark water surface from above, mostly black-blue with faint moon white reflections.",
    "ruin": "Ancient cracked stone temple floor, broken tiles with moss, faint gold rune fragments.",
    "road": "Worn dirt path from above, light brown packed earth with cart wheel tracks.",
    "ash": "Burned ground with grey ash, scattered black charred pieces.",
}


def gen_one(name: str, extras: str):
    out_path = OUT_DIR / f"{name}.png"
    if out_path.exists():
        return name, True, "skip"
    full = f"{PROMPT_PREFIX} Top-down view of {extras}"
    cmd = [
        sys.executable,
        str(GENERATE),
        "-r", "1:1",
        "--size", "2k",
        "-o", str(OUT_DIR),
        "-n", name,
        "--no-preview",
        full,
    ]
    t0 = time.time()
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
    if proc.returncode != 0:
        return name, False, proc.stderr[-200:]
    return name, True, f"{time.time()-t0:.1f}s"


with concurrent.futures.ThreadPoolExecutor(max_workers=3) as ex:
    futs = [ex.submit(gen_one, n, e) for n, e in TILES.items()]
    total = len(futs)
    for i, fut in enumerate(concurrent.futures.as_completed(futs), 1):
        n, ok, msg = fut.result()
        print(f"[{i}/{total}] {n}: {'OK' if ok else 'FAIL'} {msg}")
