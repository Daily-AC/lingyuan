import { Container, Graphics, Sprite } from 'pixi.js';
import type { TileKind, TileMsg } from '../types';
import { getCached, tryLoad } from './sprite-cache';

export const TILE_SIZE = 32;

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
  private readonly fallbackGraphics: Graphics;
  private readonly spriteContainer: Container;
  private lastTiles: TileMsg[] = [];

  constructor() {
    this.container = new Container();
    this.container.label = 'tile-layer';
    this.fallbackGraphics = new Graphics();
    this.spriteContainer = new Container();
    this.spriteContainer.label = 'tile-sprites';
    this.container.addChild(this.fallbackGraphics);
    this.container.addChild(this.spriteContainer);
  }

  setTiles(tiles: TileMsg[]): void {
    this.lastTiles = tiles;
    this.render();
  }

  private render(): void {
    const fb = this.fallbackGraphics;
    fb.clear();
    this.spriteContainer.removeChildren();
    let loadedAny = false;
    for (const t of this.lastTiles) {
      const tex = getCached('tile', t.kind);
      if (tex !== null) {
        const s = new Sprite(tex);
        s.x = t.pos.x * TILE_SIZE;
        s.y = t.pos.y * TILE_SIZE;
        s.width = TILE_SIZE;
        s.height = TILE_SIZE;
        this.spriteContainer.addChild(s);
        loadedAny = true;
      } else {
        tryLoad('tile', t.kind, () => this.render());
        const color = TILE_COLORS[t.kind];
        fb.rect(t.pos.x * TILE_SIZE, t.pos.y * TILE_SIZE, TILE_SIZE, TILE_SIZE).fill(color);
      }
    }
    if (loadedAny && this.spriteContainer.children.length === this.lastTiles.length) {
      // 所有 tile 都用上 sprite，把 fallback 清掉
      fb.clear();
    }
  }
}
