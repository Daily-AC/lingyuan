// Server message types, mirroring crates/server/src/routes/ws.rs and
// crates/server/src/state.rs (snake_case serde).

export interface TileCoord {
  x: number;
  y: number;
}

export type Biome = 'qingzhu' | 'cangsong' | 'yueze' | 'zhuyang' | 'heishi';

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

export type AgentRuntimeState = 'alive' | 'dying' | 'meditating';

export interface ItemStack {
  item: string;
  n: number;
}

export interface SpectatorAgent {
  id: string;
  name: string;
  pos: TileCoord;
  hp: number;
  hunger: number;
  stamina: number;
  state: AgentRuntimeState;
  inventory: ItemStack[];
}

export interface SpectatorEntity {
  pos: TileCoord;
  /// Format: "plant:mushroom" | "drop:stone" | "building:campfire" | "creature:wolf"
  kind: string;
  label: string | null;
  id: number | null;
}

export type Season = 'chun' | 'xia' | 'qiu' | 'dong';

export type TickEvent =
  | { kind: 'agent_joined'; data: { agent: string; name: string; at: TileCoord } }
  | { kind: 'agent_left'; data: { agent: string; name: string } }
  | { kind: 'agent_moved'; data: { agent: string; from: TileCoord; to: TileCoord } }
  | { kind: 'agent_move_failed'; data: { agent: string; reason: string } }
  | { kind: 'agent_gathered'; data: { agent: string; item: string; n: number; from: TileCoord } }
  | { kind: 'agent_gather_failed'; data: { agent: string; reason: string } }
  | { kind: 'agent_ate'; data: { agent: string; item: string; hp_gain: number; hunger_gain: number } }
  | { kind: 'agent_crafted'; data: { agent: string; recipe: string } }
  | { kind: 'agent_craft_failed'; data: { agent: string; reason: string } }
  | { kind: 'agent_placed'; data: { agent: string; building: string; at: TileCoord } }
  | { kind: 'agent_picked_up'; data: { agent: string; item: string; n: number } }
  | { kind: 'agent_dropped'; data: { agent: string; item: string; n: number } }
  | { kind: 'agent_died'; data: { agent: string; at: TileCoord; cause: string } }
  | { kind: 'agent_respawned'; data: { agent: string; at: TileCoord } }
  | { kind: 'agent_attacked_agent'; data: { attacker: string; target: string; damage: number; weapon: string | null } }
  | { kind: 'agent_attacked_creature'; data: { attacker: string; creature_id: number; damage: number } }
  | { kind: 'agent_attack_failed'; data: { agent: string; reason: string } }
  | { kind: 'creature_spawned'; data: { id: number; kind: string; at: TileCoord } }
  | { kind: 'creature_killed'; data: { id: number; kind: string; at: TileCoord } }
  | { kind: 'creature_attacked_agent'; data: { creature_id: number; creature_kind: string; target: string; damage: number } }
  | { kind: 'boss_spawned'; data: { id: number; kind: string; at: TileCoord; announcement: string } }
  | { kind: 'boss_killed'; data: { id: number; kind: string; slayer: string | null; at: TileCoord } }
  | { kind: 'agent_wrote_sign'; data: { agent: string; pos: TileCoord; text_excerpt: string } }
  | { kind: 'agent_sent_mail'; data: { from: string; to: string; text_excerpt: string } }
  | { kind: 'season_changed'; data: { to: Season } }
  | { kind: 'day_started'; data: { day: number } }
  | { kind: 'night_started'; data: { day: number } };

export interface SpectatorView {
  tick: number;
  clock: WorldClock;
  agents: SpectatorAgent[];
  entities: SpectatorEntity[];
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
  entities: SpectatorEntity[];
}

export interface TickMsg {
  kind: 'tick';
  view: SpectatorView;
}

export type ServerMsg = SnapshotMsg | TickMsg;
