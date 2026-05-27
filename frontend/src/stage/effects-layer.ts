import { Container, Graphics, Text, Ticker } from 'pixi.js';

interface FloatingText {
  text: Text;
  born: number;
  lifetime: number;
}

interface InkBlob {
  gfx: Graphics;
  born: number;
  lifetime: number;
  worldX: number;
  worldY: number;
  maxR: number;
}

export interface DamageEffect {
  worldX: number;
  worldY: number;
  label: string;
  color: number;
}

/** 浮字层：伤害数字 / 死亡水墨 puff 等暂时性视觉特效 */
export class EffectsLayer {
  readonly container: Container;
  private readonly active: FloatingText[] = [];
  private readonly inks: InkBlob[] = [];
  private tickerStarted = false;

  constructor() {
    this.container = new Container();
    this.container.label = 'effects-layer';
  }

  pushInkWash(worldX: number, worldY: number, maxR: number = 40): void {
    const gfx = new Graphics();
    gfx.x = worldX;
    gfx.y = worldY;
    this.container.addChild(gfx);
    this.inks.push({ gfx, born: performance.now(), lifetime: 1600, worldX, worldY, maxR });
    if (!this.tickerStarted) {
      this.tickerStarted = true;
      Ticker.shared.add(this.animate, this);
    }
  }

  push(e: DamageEffect): void {
    const t = new Text({
      text: e.label,
      style: {
        fontFamily: 'system-ui, -apple-system, "PingFang SC", sans-serif',
        fontSize: 14,
        fontWeight: 'bold',
        fill: e.color,
        stroke: { color: 0x2a2826, width: 3, alpha: 0.9 },
      },
    });
    t.anchor.set(0.5, 1);
    t.x = e.worldX;
    t.y = e.worldY;
    this.container.addChild(t);
    this.active.push({ text: t, born: performance.now(), lifetime: 1200 });
    if (!this.tickerStarted) {
      this.tickerStarted = true;
      Ticker.shared.add(this.animate, this);
    }
  }

  private animate = (): void => {
    const now = performance.now();
    for (let i = this.active.length - 1; i >= 0; i--) {
      const e = this.active[i]!;
      const age = now - e.born;
      if (age >= e.lifetime) {
        this.container.removeChild(e.text);
        e.text.destroy();
        this.active.splice(i, 1);
        continue;
      }
      const t = age / e.lifetime;
      e.text.y -= 0.5;
      e.text.alpha = 1 - t * t;
    }
    // ink wash
    for (let i = this.inks.length - 1; i >= 0; i--) {
      const k = this.inks[i]!;
      const age = now - k.born;
      if (age >= k.lifetime) {
        this.container.removeChild(k.gfx);
        k.gfx.destroy();
        this.inks.splice(i, 1);
        continue;
      }
      const t = age / k.lifetime;
      const r = k.maxR * Math.sqrt(t); // 半径开根号扩散
      const alpha = (1 - t) * 0.85;
      k.gfx.clear();
      // 多层水墨晕：3 个同心圆，靠外更淡
      for (let layer = 3; layer >= 1; layer--) {
        const rr = r * (layer / 3);
        k.gfx.circle(0, 0, rr).fill({ color: 0x2a2826, alpha: alpha * (0.3 + (3 - layer) * 0.2) });
      }
    }
  };
}
