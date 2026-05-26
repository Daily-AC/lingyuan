import { Container, Graphics, Text } from 'pixi.js';
import type { SpectatorEntity } from '../types';

// kind 字符串 -> 颜色 / 形状
// "plant:mushroom" -> ['plant','mushroom']
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

export class EntityLayer {
  container = new Container();

  render(entities: SpectatorEntity[], tileSize: number) {
    this.container.removeChildren();
    const g = new Graphics();
    for (const e of entities) {
      const [category, sub] = e.kind.split(':');
      const cx = e.pos.x * tileSize + tileSize / 2;
      const cy = e.pos.y * tileSize + tileSize / 2;
      let color = 0xff00ff;
      if (category === 'plant') {
        color = PLANT_COLOR[sub] ?? 0x90c090;
        g.circle(cx, cy, tileSize * 0.25).fill(color);
      } else if (category === 'drop') {
        color = DROP_COLOR[sub] ?? 0xf2efe4;
        g.rect(cx - tileSize * 0.3, cy - tileSize * 0.3, tileSize * 0.6, tileSize * 0.6).fill(color);
        g.rect(cx - tileSize * 0.3, cy - tileSize * 0.3, tileSize * 0.6, tileSize * 0.6).stroke({
          color: 0x2a2826,
          width: 1,
        });
      } else if (category === 'building') {
        color = BUILDING_COLOR[sub] ?? 0xd9a441;
        const r = tileSize * 0.42;
        g.rect(cx - r, cy - r, r * 2, r * 2).fill(color);
        g.rect(cx - r, cy - r, r * 2, r * 2).stroke({ color: 0xf2efe4, width: 1 });
      }
    }
    this.container.addChild(g);
    // 可选 label（如 "×3"）
    for (const e of entities) {
      if (!e.label) continue;
      const t = new Text({
        text: e.label,
        style: { fontSize: 7, fill: 0xf2efe4, fontFamily: 'monospace' },
      });
      t.x = e.pos.x * tileSize + tileSize * 0.5;
      t.y = e.pos.y * tileSize + tileSize * 0.2;
      this.container.addChild(t);
    }
  }
}
