import { Container, Text, Ticker } from 'pixi.js';

interface FloatingText {
  text: Text;
  born: number;
  lifetime: number;
}

export interface DamageEffect {
  worldX: number;
  worldY: number;
  label: string;
  color: number;
}

/** 浮字层：伤害数字 / 死亡 puff 等暂时性视觉特效 */
export class EffectsLayer {
  readonly container: Container;
  private readonly active: FloatingText[] = [];
  private tickerStarted = false;

  constructor() {
    this.container = new Container();
    this.container.label = 'effects-layer';
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
  };
}
