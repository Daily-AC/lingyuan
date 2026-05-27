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
      // drop:X 走 item sprite（drops 就是 item 形态）
      const spriteCategory = category === 'drop' ? 'item' : category;
      const tex = getCached(spriteCategory, sub);
      if (tex !== null) {
        const s = new Sprite(tex);
        s.anchor.set(0.5);
        s.x = cx;
        s.y = cy;
        s.width = category === 'drop' ? tileSize * 0.8 : tileSize;
        s.height = category === 'drop' ? tileSize * 0.8 : tileSize;
        this.container.addChild(s);
        continue;
      }
      if (this.redrawHook !== null) {
        tryLoad(spriteCategory, sub, this.redrawHook);
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
      } else if (category === 'sign') {
        // 路牌：金黄小杆 + 上方布幡
        fallbackG.rect(cx - 1, cy - tileSize * 0.5, 2, tileSize * 0.7).fill({ color: 0x6b4a2e });
        fallbackG
          .rect(cx - tileSize * 0.32, cy - tileSize * 0.55, tileSize * 0.64, tileSize * 0.28)
          .fill({ color: 0xd9a441 })
          .stroke({ color: 0x2a2826, width: 1 });
      }
    }
    if (usedFallback) {
      this.container.addChild(fallbackG);
    }
    // creature hp 条
    const barG = new Graphics();
    let anyBar = false;
    for (const e of entities) {
      if (!e.kind.startsWith('creature:') || e.label === null) continue;
      const m = e.label.match(/^(\d+)\/(\d+)$/);
      if (m === null) continue;
      const cur = parseInt(m[1]!, 10);
      const max = parseInt(m[2]!, 10);
      if (max <= 0) continue;
      const ratio = Math.max(0, Math.min(1, cur / max));
      const bx = e.pos.x * tileSize + tileSize * 0.1;
      const by = e.pos.y * tileSize - 1;
      const bw = tileSize * 0.8;
      const bh = 3;
      barG.rect(bx, by, bw, bh).fill({ color: 0x2a2826, alpha: 0.8 });
      barG.rect(bx, by, bw * ratio, bh).fill({ color: 0xb83a2e });
      anyBar = true;
    }
    if (anyBar) this.container.addChild(barG);
    // 其他实体的文字 label（drop 数量、sign 文本），creature 已上 hp 条不再画字
    for (const e of entities) {
      if (e.label === null) continue;
      if (e.kind.startsWith('creature:')) continue;
      const isSign = e.kind.startsWith('sign:');
      const t = new Text({
        text: e.label,
        style: {
          fontSize: isSign ? 9 : 7,
          fill: isSign ? 0xf2efe4 : 0xf2efe4,
          fontFamily: 'system-ui',
          stroke: isSign ? { color: 0x2a2826, width: 3 } : undefined,
        },
      });
      if (isSign) {
        t.anchor.set(0.5, 1);
        t.x = e.pos.x * tileSize + tileSize / 2;
        t.y = e.pos.y * tileSize - 4;
      } else {
        t.x = e.pos.x * tileSize + tileSize * 0.55;
        t.y = e.pos.y * tileSize + tileSize * 0.05;
      }
      this.container.addChild(t);
    }
  }
}
