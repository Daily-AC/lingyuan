import { Container, Graphics, Sprite, Text } from 'pixi.js';
import type { SpectatorAgent } from '../types';
import { TILE_SIZE } from './tile-layer';
import { getCached, tryLoad } from './sprite-cache';

function hashTint(id: string): number {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < id.length; i++) {
    h ^= id.charCodeAt(i);
    h = Math.imul(h, 16777619) >>> 0;
  }
  const r = 0x80 | (h & 0x7f);
  const g = 0x80 | ((h >>> 8) & 0x7f);
  const b = 0x80 | ((h >>> 16) & 0x7f);
  return (r << 16) | (g << 8) | b;
}

export class AgentLayer {
  readonly container: Container;
  private redrawHook: (() => void) | null = null;

  constructor() {
    this.container = new Container();
    this.container.label = 'agent-layer';
  }

  setRedrawHook(fn: () => void) {
    this.redrawHook = fn;
  }

  setAgents(agents: SpectatorAgent[]): void {
    this.container.removeChildren();

    for (const a of agents) {
      const cx = a.pos.x * TILE_SIZE + TILE_SIZE / 2;
      const cy = a.pos.y * TILE_SIZE + TILE_SIZE / 2;
      const tint = hashTint(a.id);

      // 优先用 sprite（朝南静态版）
      const tex = getCached('agent', 'default_south');
      if (tex !== null) {
        const s = new Sprite(tex);
        s.anchor.set(0.5);
        s.x = cx;
        s.y = cy;
        s.width = TILE_SIZE;
        s.height = TILE_SIZE;
        // 用 tint 弱叠加区分不同 agent
        s.tint = tint;
        this.container.addChild(s);
      } else {
        if (this.redrawHook !== null) tryLoad('agent', 'default_south', this.redrawHook);
        const dot = new Graphics();
        dot
          .circle(cx, cy, TILE_SIZE * 0.45)
          .fill(tint)
          .stroke({ color: 0x2a2826, width: 1, alpha: 0.85 });
        this.container.addChild(dot);
      }

      const label = new Text({
        text: a.name,
        style: {
          fontFamily: 'system-ui, -apple-system, "PingFang SC", "Microsoft YaHei", sans-serif',
          fontSize: 10,
          fill: 0xf2efe4,
          stroke: { color: 0x2a2826, width: 3, alpha: 0.85 },
        },
      });
      label.anchor.set(0.5, 1);
      label.x = cx;
      label.y = cy - TILE_SIZE * 0.55;
      this.container.addChild(label);
    }
  }
}
