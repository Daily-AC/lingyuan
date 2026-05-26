import { Container, Graphics } from 'pixi.js';
import type { TileKind, TileMsg } from '../types';

export const TILE_SIZE = 12;

const TILE_COLORS: Record<TileKind, number> = {
  grass: 0x5c8c6a,
  bamboo_forest: 0x3f6e4d,
  pine_forest: 0x2e5c3f,
  reed: 0x8fa76a,
  maple: 0xb83a2e,
  sand: 0xd9a441,
  stone: 0x6f6a60,
  mountain: 0x3a3632,
  shallow_water: 0x4a7a8c,
  deep_water: 0x2a5260,
  ruin: 0x5c4a3e,
  road: 0x8c7a5c,
  ash: 0x2a2826,
};

export class TileLayer {
  readonly container: Container;
  private readonly graphics: Graphics;

  constructor() {
    this.container = new Container();
    this.container.label = 'tile-layer';
    this.graphics = new Graphics();
    this.container.addChild(this.graphics);
  }

  setTiles(tiles: TileMsg[]): void {
    const g = this.graphics;
    g.clear();
    for (const t of tiles) {
      const color = TILE_COLORS[t.kind];
      g.rect(t.pos.x * TILE_SIZE, t.pos.y * TILE_SIZE, TILE_SIZE, TILE_SIZE).fill(color);
    }
  }
}
