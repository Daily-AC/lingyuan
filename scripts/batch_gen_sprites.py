#!/usr/bin/env python3
"""
批量调 ~/.claude/skills/gpt-image-2/tools/generate.py 生成所有 sprite。
并行 N 路。失败的 sprite 列在最后。
"""
from __future__ import annotations

import argparse
import concurrent.futures
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from gen_sprites import MANIFEST, BASE_PROMPT  # noqa: E402

GENERATE = Path.home() / ".claude" / "skills" / "gpt-image-2" / "tools" / "generate.py"
OUT_RAW = Path(__file__).parent.parent / "assets" / "sprites_raw"


def gen_one(category: str, name: str, extras: str, skip_existing: bool) -> tuple[str, str, bool, str]:
    out_dir = OUT_RAW / category
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / f"{name}.png"
    if skip_existing and out_path.exists():
        return (category, name, True, "skip (exists)")
    prompt = BASE_PROMPT.format(EXTRAS=extras)
    cmd = [
        sys.executable,
        str(GENERATE),
        "-r",
        "1:1",
        "--size",
        "2k",
        "-o",
        str(out_dir),
        "-n",
        name,
        "--no-preview",
        prompt,
    ]
    t0 = time.time()
    try:
        proc = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=900,
        )
    except subprocess.TimeoutExpired:
        return (category, name, False, "timeout 900s")
    dur = time.time() - t0
    if proc.returncode != 0:
        return (category, name, False, f"rc={proc.returncode} stderr={proc.stderr[-200:]}")
    return (category, name, True, f"ok {dur:.1f}s")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("-j", "--parallel", type=int, default=3)
    ap.add_argument("--only-category", help="只跑某个 category")
    ap.add_argument("--limit", type=int, default=0, help="最多跑几张（调试用）")
    ap.add_argument("--skip-existing", action="store_true", default=True)
    args = ap.parse_args()

    if not GENERATE.exists():
        print(f"❌ generate.py 不在 {GENERATE}", file=sys.stderr)
        return 2

    items = list(MANIFEST)
    if args.only_category:
        items = [t for t in items if t[0] == args.only_category]
    if args.limit:
        items = items[: args.limit]

    print(f"待生成 {len(items)} 张，并行 {args.parallel}")
    failed = []
    done = 0
    t_start = time.time()
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.parallel) as ex:
        futs = {
            ex.submit(gen_one, c, n, e, args.skip_existing): (c, n)
            for c, n, e in items
        }
        for fut in concurrent.futures.as_completed(futs):
            c, n, ok, msg = fut.result()
            done += 1
            print(f"[{done}/{len(items)}] {c}/{n}: {msg}")
            if not ok:
                failed.append(f"{c}/{n}: {msg}")
    print(f"\n总耗时 {time.time() - t_start:.1f}s")
    if failed:
        print(f"❌ {len(failed)} 张失败：")
        for f in failed:
            print(f"  {f}")
        return 1
    print(f"✅ {done}/{len(items)} 全部成功")
    return 0


if __name__ == "__main__":
    sys.exit(main())
