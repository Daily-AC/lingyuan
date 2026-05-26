import { Container, Graphics, Text } from 'pixi.js';
import type { SpectatorAgent } from '../types';
import { TILE_SIZE } from './tile-layer';

function hashTint(id: string): number {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < id.length; i++) {
    h ^= id.charCodeAt(i);
    h = Math.imul(h, 16777619) >>> 0;
  }
  // Bias towards bright/saturated palette so dots pop against terrain.
  const r = 0x80 | (h & 0x7f);
  const g = 0x80 | ((h >>> 8) & 0x7f);
  const b = 0x80 | ((h >>> 16) & 0x7f);
  return (r << 16) | (g << 8) | b;
}

export class AgentLayer {
  readonly container: Container;

  constructor() {
    this.container = new Container();
    this.container.label = 'agent-layer';
  }

  setAgents(agents: SpectatorAgent[]): void {
    this.container.removeChildren();

    for (const a of agents) {
      const cx = a.pos.x * TILE_SIZE + TILE_SIZE / 2;
      const cy = a.pos.y * TILE_SIZE + TILE_SIZE / 2;
      const tint = hashTint(a.id);

      const dot = new Graphics();
      dot
        .circle(cx, cy, TILE_SIZE * 0.45)
        .fill(tint)
        .stroke({ color: 0x2a2826, width: 1, alpha: 0.85 });
      this.container.addChild(dot);

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
