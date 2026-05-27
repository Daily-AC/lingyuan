// Lazy sprite texture cache + fallback indicator.
// 调 get('plant', 'mushroom') -> Texture | null（null = 未找到，回退到色块）

import { Assets, Texture } from 'pixi.js';

type CacheEntry = { texture: Texture | null; loading: Promise<Texture | null> | null };
const cache = new Map<string, CacheEntry>();
const failed = new Set<string>();

function key(category: string, name: string): string {
  return `${category}/${name}`;
}

export function getCached(category: string, name: string): Texture | null {
  const k = key(category, name);
  const e = cache.get(k);
  if (e && e.texture !== null) return e.texture;
  return null;
}

/**
 * 同步发起加载，返回 null（首次）。加载完成后下次 getCached 命中。
 * 后续 tick 重绘自然就用上 sprite。
 */
export function tryLoad(category: string, name: string, onReady: () => void): void {
  const k = key(category, name);
  if (failed.has(k)) return;
  const e = cache.get(k);
  if (e !== undefined) return; // 已在加载或已就绪
  const url = `/sprites/${category}/${name}.png`;
  const entry: CacheEntry = { texture: null, loading: null };
  cache.set(k, entry);
  entry.loading = Assets.load(url)
    .then((t: Texture) => {
      // 像素艺术：禁用平滑
      t.source.scaleMode = 'nearest';
      entry.texture = t;
      onReady();
      return t;
    })
    .catch(() => {
      failed.add(k);
      cache.delete(k);
      return null;
    });
}
