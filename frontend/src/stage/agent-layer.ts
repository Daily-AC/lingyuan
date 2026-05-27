import { Container, Graphics, Rectangle, Sprite, Text, Ticker } from 'pixi.js';
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

function hashPhase(id: string): number {
  let h = 0;
  for (let i = 0; i < id.length; i++) {
    h = (h * 31 + id.charCodeAt(i)) >>> 0;
  }
  return (h % 1000) / 1000;
}

interface AgentRenderState {
  agent: SpectatorAgent;
  container: Container;
  groundHalo: Graphics;
  spriteOrDot: Sprite | Graphics;
  baseSprite: Sprite | null;
  selectionRing: Graphics;
  baseY: number;
  prevX: number;
  prevY: number;
  targetX: number;
  targetY: number;
  lerpStartMs: number;
}

const LERP_MS = 220;

export class AgentLayer {
  readonly container: Container;
  private redrawHook: (() => void) | null = null;
  private readonly states = new Map<string, AgentRenderState>();
  private selectedId: string | null = null;
  private tickerStarted = false;
  private clickHandler: ((id: string) => void) | null = null;

  constructor() {
    this.container = new Container();
    this.container.label = 'agent-layer';
  }

  setRedrawHook(fn: () => void): void {
    this.redrawHook = fn;
  }

  onAgentClicked(fn: (id: string) => void): void {
    this.clickHandler = fn;
  }

  setSelected(id: string | null): void {
    this.selectedId = id;
    for (const [aid, s] of this.states) {
      s.selectionRing.visible = aid === id;
    }
  }

  setAgents(agents: SpectatorAgent[]): void {
    const seen = new Set<string>();
    for (const a of agents) {
      seen.add(a.id);
      let st = this.states.get(a.id);
      if (st === undefined) {
        st = this.create(a);
        this.states.set(a.id, st);
        this.container.addChild(st.container);
      } else {
        st.agent = a;
      }
      this.position(st);
    }
    // 移除离场的
    for (const [aid, s] of [...this.states.entries()]) {
      if (!seen.has(aid)) {
        this.container.removeChild(s.container);
        s.container.destroy({ children: true });
        this.states.delete(aid);
      }
    }
    if (!this.tickerStarted) {
      this.tickerStarted = true;
      Ticker.shared.add(this.animate, this);
    }
  }

  private create(a: SpectatorAgent): AgentRenderState {
    const container = new Container();
    container.label = `agent-${a.id}`;
    const tint = hashTint(a.id);
    container.eventMode = 'static';
    container.cursor = 'pointer';
    container.hitArea = new Rectangle(
      -TILE_SIZE * 0.7,
      -TILE_SIZE * 0.9,
      TILE_SIZE * 1.4,
      TILE_SIZE * 1.6,
    );
    const captured = a.id;
    container.on('pointerdown', (ev) => {
      ev.stopPropagation();
      if (this.clickHandler !== null) this.clickHandler(captured);
    });

    // 脚下光环（恒亮，半透月白）
    const groundHalo = new Graphics();
    groundHalo
      .ellipse(0, TILE_SIZE * 0.4, TILE_SIZE * 0.45, TILE_SIZE * 0.15)
      .fill({ color: 0xf2efe4, alpha: 0.35 });
    container.addChild(groundHalo);

    // 选中圆环（默认隐藏，被选中时显示 + 呼吸）
    const selectionRing = new Graphics();
    selectionRing
      .circle(0, 0, TILE_SIZE * 0.65)
      .stroke({ color: 0xd9a441, width: 2.5, alpha: 0.9 });
    selectionRing.visible = false;
    container.addChild(selectionRing);

    // sprite（真 wuxia 修士）or fallback dot
    const tex = getCached('agent', 'default_south');
    let spriteOrDot: Sprite | Graphics;
    let baseSprite: Sprite | null = null;
    if (tex !== null) {
      const sp = new Sprite(tex);
      sp.anchor.set(0.5);
      sp.width = TILE_SIZE * 1.1;
      sp.height = TILE_SIZE * 1.1;
      sp.tint = tint;
      container.addChild(sp);
      spriteOrDot = sp;
      baseSprite = sp;
    } else {
      if (this.redrawHook !== null) tryLoad('agent', 'default_south', this.redrawHook);
      const dot = new Graphics();
      dot
        .circle(0, 0, TILE_SIZE * 0.45)
        .fill(tint)
        .stroke({ color: 0x2a2826, width: 1.5, alpha: 0.9 });
      container.addChild(dot);
      spriteOrDot = dot;
    }

    // 名字胶囊：先建文本量尺寸，再画背景，再 add
    const labelText = new Text({
      text: a.name,
      style: {
        fontFamily: 'system-ui, -apple-system, "PingFang SC", "Microsoft YaHei", sans-serif',
        fontSize: 12,
        fill: 0xf2efe4,
        fontWeight: 'bold',
      },
    });
    labelText.anchor.set(0.5, 1);
    labelText.x = 0;
    labelText.y = -TILE_SIZE * 0.7;
    const padX = 6;
    const padY = 2;
    const w = labelText.width + padX * 2;
    const h = labelText.height + padY * 2;
    const pill = new Graphics();
    pill
      .roundRect(-w / 2, labelText.y - h, w, h, 4)
      .fill({ color: 0x2a2826, alpha: 0.85 })
      .stroke({ color: 0xd9a441, width: 1, alpha: 0.7 });
    container.addChild(pill);
    container.addChild(labelText);

    const cx = a.pos.x * TILE_SIZE + TILE_SIZE / 2;
    const cy = a.pos.y * TILE_SIZE + TILE_SIZE / 2;
    container.x = cx;
    container.y = cy;
    return {
      agent: a,
      container,
      groundHalo,
      spriteOrDot,
      baseSprite,
      selectionRing,
      baseY: 0,
      prevX: cx,
      prevY: cy,
      targetX: cx,
      targetY: cy,
      lerpStartMs: performance.now(),
    };
  }

  private position(st: AgentRenderState): void {
    const tx = st.agent.pos.x * TILE_SIZE + TILE_SIZE / 2;
    const ty = st.agent.pos.y * TILE_SIZE + TILE_SIZE / 2;
    if (tx !== st.targetX || ty !== st.targetY) {
      // 起新 lerp：prev = 当前位置（可能 lerp 中），target = 新
      st.prevX = st.container.x;
      st.prevY = st.container.y;
      st.targetX = tx;
      st.targetY = ty;
      st.lerpStartMs = performance.now();
    }
    st.container.alpha = st.agent.state === 'alive' ? 1 : 0.55;
  }

  private animate = (): void => {
    const now = performance.now();
    const t = now / 1000;
    for (const [aid, s] of this.states) {
      // lerp 平滑移动
      const dt = now - s.lerpStartMs;
      if (dt < LERP_MS) {
        const p = dt / LERP_MS;
        const ease = p * (2 - p); // ease-out
        s.container.x = s.prevX + (s.targetX - s.prevX) * ease;
        s.container.y = s.prevY + (s.targetY - s.prevY) * ease;
      } else {
        s.container.x = s.targetX;
        s.container.y = s.targetY;
      }
      // idle bob: 浮动 ±1.5 px
      const phase = hashPhase(aid) * Math.PI * 2;
      const bob = Math.sin(t * 1.6 + phase) * 1.5;
      if (s.baseSprite !== null) {
        s.baseSprite.y = bob;
      } else {
        s.spriteOrDot.y = bob;
      }
      // 选中环呼吸：alpha + scale 微缩放
      if (s.agent.id === this.selectedId) {
        const breath = 0.55 + 0.4 * (Math.sin(t * 2.4) * 0.5 + 0.5);
        s.selectionRing.alpha = breath;
        const sc = 1 + 0.06 * Math.sin(t * 2.4);
        s.selectionRing.scale.set(sc);
      }
    }
  };

  destroy(): void {
    if (this.tickerStarted) {
      Ticker.shared.remove(this.animate, this);
      this.tickerStarted = false;
    }
  }
}
