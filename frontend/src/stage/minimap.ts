import type { SpectatorAgent, TileKind, TileMsg } from '../types';

const MAP_PX = 160;
const MAP_TILE_BG: Record<TileKind, string> = {
  grass: '#5c8c6a',
  bamboo_forest: '#3f6e4d',
  pine_forest: '#2e5c3f',
  reed: '#8fa76a',
  maple: '#b83a2e',
  sand: '#d9a441',
  stone: '#6f6a60',
  mountain: '#3a3632',
  shallow_water: '#4a7a8c',
  deep_water: '#2a5260',
  ruin: '#5c4a3e',
  road: '#8c7a5c',
  ash: '#2a2826',
};

export class MiniMap {
  readonly el: HTMLCanvasElement;
  private gridWidth = 0;
  private gridHeight = 0;
  private tiles: TileMsg[] = [];
  private tileBgCache: ImageData | null = null;

  constructor() {
    const c = document.createElement('canvas');
    c.id = 'minimap';
    c.width = MAP_PX;
    c.height = MAP_PX;
    this.el = c;
  }

  setGrid(width: number, height: number, tiles: TileMsg[]): void {
    this.gridWidth = width;
    this.gridHeight = height;
    this.tiles = tiles;
    this.tileBgCache = null;
    this.render([], null);
  }

  render(agents: SpectatorAgent[], focusedId: string | null): void {
    if (this.gridWidth === 0 || this.gridHeight === 0) return;
    const ctx = this.el.getContext('2d');
    if (ctx === null) return;
    const scale = MAP_PX / Math.max(this.gridWidth, this.gridHeight);
    // tile bg（缓存）
    if (this.tileBgCache === null) {
      ctx.fillStyle = '#161412';
      ctx.fillRect(0, 0, MAP_PX, MAP_PX);
      const tileSz = Math.max(1, Math.floor(scale));
      for (const t of this.tiles) {
        ctx.fillStyle = MAP_TILE_BG[t.kind] ?? '#444';
        ctx.fillRect(t.pos.x * scale, t.pos.y * scale, tileSz, tileSz);
      }
      this.tileBgCache = ctx.getImageData(0, 0, MAP_PX, MAP_PX);
    } else {
      ctx.putImageData(this.tileBgCache, 0, 0);
    }
    // agents
    for (const a of agents) {
      const x = a.pos.x * scale;
      const y = a.pos.y * scale;
      const r = a.id === focusedId ? 4 : 2.5;
      ctx.fillStyle = a.id === focusedId ? '#d9a441' : '#f2efe4';
      ctx.strokeStyle = '#2a2826';
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.arc(x, y, r, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }
  }
}
