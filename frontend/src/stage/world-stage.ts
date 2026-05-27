import { Application, Container } from 'pixi.js';
import type { SpectatorAgent, SpectatorEntity, TileMsg, TickEvent } from '../types';
import { TILE_SIZE, TileLayer } from './tile-layer';
import { AgentLayer } from './agent-layer';
import { EntityLayer } from './entity-layer';
import { EffectsLayer } from './effects-layer';

export class WorldStage {
  private readonly app: Application;
  private readonly root: Container;
  private readonly tileLayer: TileLayer;
  private readonly entityLayer: EntityLayer;
  private readonly agentLayer: AgentLayer;
  private readonly effectsLayer: EffectsLayer;
  private lastEntities: SpectatorEntity[];
  private host: HTMLElement | null;
  private gridWidth: number;
  private gridHeight: number;
  private resizeObserver: ResizeObserver | null;
  private focusedAgentId: string | null;
  private focusZoom: number;
  private lastAgents: SpectatorAgent[];

  constructor() {
    this.app = new Application();
    this.root = new Container();
    this.root.label = 'world-root';
    this.tileLayer = new TileLayer();
    this.entityLayer = new EntityLayer();
    this.agentLayer = new AgentLayer();
    this.effectsLayer = new EffectsLayer();
    this.lastEntities = [];
    this.host = null;
    this.gridWidth = 0;
    this.gridHeight = 0;
    this.resizeObserver = null;
    this.focusedAgentId = null;
    this.focusZoom = 2.5;
    this.lastAgents = [];
  }

  async mount(el: HTMLElement): Promise<void> {
    this.host = el;
    await this.app.init({
      resizeTo: el,
      antialias: false,
      background: 0x161412,
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
    });
    el.appendChild(this.app.canvas);

    this.root.addChild(this.tileLayer.container);
    this.root.addChild(this.entityLayer.container);
    this.root.addChild(this.agentLayer.container);
    this.root.addChild(this.effectsLayer.container);
    this.app.stage.addChild(this.root);

    this.resizeObserver = new ResizeObserver(() => this.handleResize());
    this.resizeObserver.observe(el);
    // 初次显式 resize 以防 init 时 host 还没渲染
    this.handleResize();
  }

  private handleResize(): void {
    if (this.host === null) return;
    const w = this.host.clientWidth;
    const h = this.host.clientHeight;
    if (w > 0 && h > 0) {
      this.app.renderer.resize(w, h);
    }
    this.reposition();
  }

  setGrid(width: number, height: number, tiles: TileMsg[]): void {
    this.gridWidth = width;
    this.gridHeight = height;
    this.tileLayer.setTiles(tiles);
    this.reposition();
  }

  setAgents(agents: SpectatorAgent[]): void {
    this.lastAgents = agents;
    this.agentLayer.setRedrawHook(() => this.agentLayer.setAgents(agents));
    this.agentLayer.setAgents(agents);
    if (this.focusedAgentId !== null) {
      const a = agents.find((x) => x.id === this.focusedAgentId);
      if (a !== undefined) {
        this.zoomTo(a.pos.x, a.pos.y, this.focusZoom);
      }
    }
  }

  setEntities(entities: SpectatorEntity[]): void {
    this.lastEntities = entities;
    this.entityLayer.setRedrawHook(() => this.entityLayer.render(entities, TILE_SIZE));
    this.entityLayer.render(entities, TILE_SIZE);
  }

  pushEvents(events: TickEvent[]): void {
    for (const e of events) {
      this.routeEvent(e);
    }
  }

  private routeEvent(e: TickEvent): void {
    const agentPos = (name: string) => {
      const a = this.lastAgents.find((x) => x.name === name || x.id === name);
      return a ? this.tileWorldCenter(a.pos.x, a.pos.y) : null;
    };
    const creaturePos = (id: number) => {
      const c = this.lastEntities.find((x) => x.id === id);
      return c ? this.tileWorldCenter(c.pos.x, c.pos.y) : null;
    };
    switch (e.kind) {
      case 'agent_attacked_agent': {
        const p = agentPos(e.data.target);
        if (p) this.effectsLayer.push({ worldX: p[0], worldY: p[1], label: `-${e.data.damage}`, color: 0xb83a2e });
        break;
      }
      case 'creature_attacked_agent': {
        const p = agentPos(e.data.target);
        if (p) this.effectsLayer.push({ worldX: p[0], worldY: p[1], label: `-${e.data.damage}`, color: 0xb83a2e });
        break;
      }
      case 'agent_attacked_creature': {
        const p = creaturePos(e.data.creature_id);
        if (p) this.effectsLayer.push({ worldX: p[0], worldY: p[1], label: `-${e.data.damage}`, color: 0xd9a441 });
        break;
      }
      case 'agent_ate': {
        const p = agentPos(e.data.agent);
        if (p) this.effectsLayer.push({ worldX: p[0], worldY: p[1] - 6, label: `+${e.data.hunger_gain}饱`, color: 0x5c8c6a });
        break;
      }
      case 'agent_gathered': {
        const p = agentPos(e.data.agent);
        if (p) this.effectsLayer.push({ worldX: p[0], worldY: p[1] - 6, label: `+${e.data.n} ${e.data.item}`, color: 0xf2efe4 });
        break;
      }
      case 'agent_died': {
        const [wx, wy] = this.tileWorldCenter(e.data.at.x, e.data.at.y);
        this.effectsLayer.push({ worldX: wx, worldY: wy, label: '殁', color: 0xb83a2e });
        break;
      }
      case 'boss_spawned': {
        const [wx, wy] = this.tileWorldCenter(e.data.at.x, e.data.at.y);
        this.effectsLayer.push({ worldX: wx, worldY: wy, label: `※ ${e.data.announcement}`, color: 0xb83a2e });
        break;
      }
      default:
        break;
    }
  }

  private tileWorldCenter(tx: number, ty: number): [number, number] {
    return [tx * TILE_SIZE + TILE_SIZE / 2, ty * TILE_SIZE + TILE_SIZE / 2];
  }

  /** 切换关注模式；null = 全图，否则跟焦该 agent */
  focusAgent(id: string | null): void {
    this.focusedAgentId = id;
    this.agentLayer.setSelected(id);
    if (id !== null) {
      const a = this.lastAgents.find((x) => x.id === id);
      if (a !== undefined) {
        this.zoomTo(a.pos.x, a.pos.y, this.focusZoom);
        return;
      }
    }
    this.fit();
  }

  isFocused(): boolean {
    return this.focusedAgentId !== null;
  }

  focusedId(): string | null {
    return this.focusedAgentId;
  }

  private reposition(): void {
    if (this.focusedAgentId !== null) {
      const a = this.lastAgents.find((x) => x.id === this.focusedAgentId);
      if (a !== undefined) {
        this.zoomTo(a.pos.x, a.pos.y, this.focusZoom);
        return;
      }
    }
    this.fit();
  }

  private viewSizeCss(): [number, number] {
    if (this.host !== null) {
      return [this.host.clientWidth, this.host.clientHeight];
    }
    return [this.app.renderer.width, this.app.renderer.height];
  }

  private zoomTo(tileX: number, tileY: number, scale: number): void {
    const wx = tileX * TILE_SIZE + TILE_SIZE / 2;
    const wy = tileY * TILE_SIZE + TILE_SIZE / 2;
    const [vw, vh] = this.viewSizeCss();
    this.root.scale.set(scale);
    this.root.x = vw / 2 - wx * scale;
    this.root.y = vh / 2 - wy * scale;
  }

  private fit(): void {
    if (this.host === null || this.gridWidth === 0 || this.gridHeight === 0) {
      return;
    }
    const worldPxW = this.gridWidth * TILE_SIZE;
    const worldPxH = this.gridHeight * TILE_SIZE;
    const [viewW, viewH] = this.viewSizeCss();
    if (viewW <= 0 || viewH <= 0) {
      return;
    }
    const scale = Math.min(viewW / worldPxW, viewH / worldPxH);
    this.root.scale.set(scale);
    this.root.x = (viewW - worldPxW * scale) / 2;
    this.root.y = (viewH - worldPxH * scale) / 2;
  }
}
