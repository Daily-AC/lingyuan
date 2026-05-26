// Server message types, mirroring crates/server/src/routes/ws.rs and
// crates/server/src/state.rs (snake_case serde).

export interface TileCoord {
  x: number;
  y: number;
}

export type Biome =
  | 'qingzhu'
  | 'cangsong'
  | 'yueze'
  | 'zhuyang'
  | 'heishi';

export type TileKind =
  | 'grass'
  | 'bamboo_forest'
  | 'pine_forest'
  | 'reed'
  | 'maple'
  | 'sand'
  | 'stone'
  | 'mountain'
  | 'shallow_water'
  | 'deep_water'
  | 'ruin'
  | 'road'
  | 'ash';

export interface TileMsg {
  pos: TileCoord;
  kind: TileKind;
  biome: Biome;
}

export interface WorldClock {
  tick: number;
}

export interface SpectatorAgent {
  id: string;
  name: string;
  pos: TileCoord;
  hp: number;
}

export type Season = 'chun' | 'xia' | 'qiu' | 'dong';

export type TickEvent =
  | {
      kind: 'agent_joined';
      data: { agent: string; name: string; at: TileCoord };
    }
  | {
      kind: 'agent_left';
      data: { agent: string; name: string };
    }
  | {
      kind: 'agent_moved';
      data: { agent: string; from: TileCoord; to: TileCoord };
    }
  | {
      kind: 'agent_move_failed';
      data: { agent: string; reason: string };
    }
  | {
      kind: 'season_changed';
      data: { to: Season };
    }
  | {
      kind: 'day_started';
      data: { day: number };
    }
  | {
      kind: 'night_started';
      data: { day: number };
    };

export interface SpectatorView {
  tick: number;
  clock: WorldClock;
  agents: SpectatorAgent[];
  events: TickEvent[];
}

export interface SnapshotMsg {
  kind: 'snapshot';
  tick: number;
  clock: WorldClock;
  grid_width: number;
  grid_height: number;
  tiles: TileMsg[];
  agents: SpectatorAgent[];
}

export interface TickMsg {
  kind: 'tick';
  view: SpectatorView;
}

export type ServerMsg = SnapshotMsg | TickMsg;
