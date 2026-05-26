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

    this.resizeObserver = new ResizeObserver(() => this.fit());
    this.resizeObserver.observe(el);
  }

  setGrid(width: number, height: number, tiles: TileMsg[]): void {
    this.gridWidth = width;
    this.gridHeight = height;
    this.tileLayer.setTiles(tiles);
    this.fit();
  }

  setAgents(agents: SpectatorAgent[]): void {
    this.agentLayer.setAgents(agents);
  }

  setEntities(entities: SpectatorEntity[]): void {
    this.entityLayer.render(entities, TILE_SIZE);
  }

  private fit(): void {
    if (this.host === null || this.gridWidth === 0 || this.gridHeight === 0) {
      return;
    }
    const worldPxW = this.gridWidth * TILE_SIZE;
    const worldPxH = this.gridHeight * TILE_SIZE;
    const viewW = this.app.renderer.width / (this.app.renderer.resolution || 1);
    const viewH = this.app.renderer.height / (this.app.renderer.resolution || 1);
    if (viewW <= 0 || viewH <= 0) {
      return;
    }
    const scale = Math.min(viewW / worldPxW, viewH / worldPxH);
    this.root.scale.set(scale);
    this.root.x = (viewW - worldPxW * scale) / 2;
    this.root.y = (viewH - worldPxH * scale) / 2;
  }
}
