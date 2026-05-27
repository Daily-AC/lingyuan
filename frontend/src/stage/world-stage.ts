import { Application, Container } from 'pixi.js';
import type { SpectatorAgent, SpectatorEntity, TileMsg } from '../types';
import { TILE_SIZE, TileLayer } from './tile-layer';
import { AgentLayer } from './agent-layer';
import { EntityLayer } from './entity-layer';

export class WorldStage {
  private readonly app: Application;
  private readonly root: Container;
  private readonly tileLayer: TileLayer;
  private readonly entityLayer: EntityLayer;
  private readonly agentLayer: AgentLayer;
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
    this.entityLayer.setRedrawHook(() => this.entityLayer.render(entities, TILE_SIZE));
    this.entityLayer.render(entities, TILE_SIZE);
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
