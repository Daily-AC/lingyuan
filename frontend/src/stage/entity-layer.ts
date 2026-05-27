import { Container, Graphics, Sprite, Text } from 'pixi.js';
import type { SpectatorEntity } from '../types';
import { getCached, tryLoad } from './sprite-cache';

// kind 字符串 -> 颜色 / 形状
const PLANT_COLOR: Record<string, number> = {
  bamboo_stalk: 0x6b9d6f,
  pine_log: 0x3e5e3e,
  stone_chunk: 0x8a8a82,
  flint_chunk: 0xd9a441,
  clay_lump: 0xa57050,
  lingzhi: 0xe24c4c,
  mushroom: 0xc46aa4,
  red_berry: 0xb83a2e,
  vine: 0x7f9a52,
  reed: 0x9fb072,
};

const DROP_COLOR: Record<string, number> = {
  bamboo: 0x6b9d6f,
  pinewood: 0x3e5e3e,
  stone: 0x8a8a82,
  flint: 0xd9a441,
  clay: 0xa57050,
  vine: 0x7f9a52,
  reed: 0x9fb072,
  lingzhi: 0xe24c4c,
  mushroom: 0xc46aa4,
  red_berry: 0xb83a2e,
  bamboo_spear: 0xb88c4a,
  stone_axe: 0xa0a0a0,
  rope: 0xc4a560,
  clay_pot: 0xa57050,
  cooked_mushroom: 0xe09a64,
  cooked_berry: 0xe06464,
  rice_cake: 0xefefcf,
  campfire_kit: 0xb84a2e,
  cooking_stove_kit: 0x8a7050,
};

const BUILDING_COLOR: Record<string, number> = {
  campfire: 0xe07a3a,
  cooking_stove: 0x8c7c5c,
};

const CREATURE_COLOR: Record<string, number> = {
  rabbit: 0xe0e0d0,
  deer: 0xa0805c,
  wolf: 0x4c4a48,
  night_demon: 0x40285a,
};

export class EntityLayer {
  container = new Container();
  /** 触发外层重画的回调；sprite 异步加载完成后调用 */
  private redrawHook: (() => void) | null = null;

  setRedrawHook(fn: () => void) {
    this.redrawHook = fn;
  }

  render(entities: SpectatorEntity[], tileSize: number) {
    this.container.removeChildren();
    const fallbackG = new Graphics();
    let usedFallback = false;
    for (const e of entities) {
      const [category, sub] = e.kind.split(':');
      const cx = e.pos.x * tileSize + tileSize / 2;
      const cy = e.pos.y * tileSize + tileSize / 2;
      // 优先尝试 sprite
      const tex = getCached(category, sub);
      if (tex !== null) {
        const s = new Sprite(tex);
        s.anchor.set(0.5);
        s.x = cx;
        s.y = cy;
        s.width = tileSize;
        s.height = tileSize;
        this.container.addChild(s);
        continue;
      }
      // 触发异步加载（一次性，下次重画会用上）
      if (this.redrawHook !== null) {
        tryLoad(category, sub, this.redrawHook);
      }
      // 兜底：色块
      usedFallback = true;
      if (category === 'plant') {
        fallbackG.circle(cx, cy, tileSize * 0.25).fill(PLANT_COLOR[sub] ?? 0x90c090);
      } else if (category === 'drop') {
        const color = DROP_COLOR[sub] ?? 0xf2efe4;
        fallbackG
          .rect(cx - tileSize * 0.3, cy - tileSize * 0.3, tileSize * 0.6, tileSize * 0.6)
          .fill(color);
        fallbackG
          .rect(cx - tileSize * 0.3, cy - tileSize * 0.3, tileSize * 0.6, tileSize * 0.6)
          .stroke({ color: 0x2a2826, width: 1 });
      } else if (category === 'building') {
        const color = BUILDING_COLOR[sub] ?? 0xd9a441;
        const r = tileSize * 0.42;
        fallbackG.rect(cx - r, cy - r, r * 2, r * 2).fill(color);
        fallbackG.rect(cx - r, cy - r, r * 2, r * 2).stroke({ color: 0xf2efe4, width: 1 });
      } else if (category === 'creature') {
        const color = CREATURE_COLOR[sub] ?? 0xb83a2e;
        const r = tileSize * 0.38;
        fallbackG.poly([cx, cy - r, cx + r, cy, cx, cy + r, cx - r, cy]).fill(color);
        fallbackG
          .poly([cx, cy - r, cx + r, cy, cx, cy + r, cx - r, cy])
          .stroke({ color: 0x2a2826, width: 1 });
      }
    }
    if (usedFallback) {
      this.container.addChild(fallbackG);
    }
    for (const e of entities) {
      if (e.label === null) continue;
      const t = new Text({
        text: e.label,
        style: { fontSize: 7, fill: 0xf2efe4, fontFamily: 'monospace' },
      });
      t.x = e.pos.x * tileSize + tileSize * 0.55;
      t.y = e.pos.y * tileSize + tileSize * 0.05;
      this.container.addChild(t);
    }
  }
}
