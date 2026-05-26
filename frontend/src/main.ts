import { WorldStage } from './stage/world-stage';
import { connect } from './ws';
import type { ServerMsg, SpectatorAgent, TickEvent } from './types';

const MAX_EVENTS = 80;
const recentEvents: { tick: number; event: TickEvent }[] = [];

function el<T extends HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (node === null) {
    throw new Error(`missing element #${id}`);
  }
  return node as T;
}

function renderClock(tick: number): void {
  const ticksPerDay = 72;
  const ticksPerSeason = ticksPerDay * 10;
  const ticksPerYear = ticksPerSeason * 4;
  const year = Math.floor(tick / ticksPerYear);
  const seasonIdx = Math.floor(tick / ticksPerSeason) % 4;
  const day = Math.floor(tick / ticksPerDay) % 10;
  const tickInDay = tick % ticksPerDay;
  const seasonLabel = ['春', '夏', '秋', '冬'][seasonIdx] ?? '?';
  el('clock').textContent =
    `第 ${year} 年 · ${seasonLabel} · 第 ${day + 1} 日 · 刻 ${tickInDay} / 72 · tick ${tick}`;
}

function renderAgents(agents: SpectatorAgent[]): void {
  const list = el<HTMLUListElement>('agent-list');
  list.replaceChildren();
  for (const a of agents) {
    const li = document.createElement('li');
    li.className = `agent-row agent-state-${a.state}`;
    const name = document.createElement('span');
    name.className = 'agent-name';
    name.textContent = a.name;
    const meta = document.createElement('span');
    meta.className = 'agent-meta';
    meta.textContent = `(${a.pos.x},${a.pos.y}) hp${a.hp} 饥${a.hunger}`;
    li.appendChild(name);
    li.appendChild(meta);
    list.appendChild(li);
  }
  if (agents.length === 0) {
    const li = document.createElement('li');
    li.className = 'empty';
    li.textContent = '尚无在世';
    list.appendChild(li);
  }
}

function describeEvent(ev: TickEvent): string {
  switch (ev.kind) {
    case 'agent_joined':
      return `${ev.data.name} 入世 @(${ev.data.at.x},${ev.data.at.y})`;
    case 'agent_left':
      return `${ev.data.name} 离世`;
    case 'agent_moved':
      return `${ev.data.agent} 移 (${ev.data.from.x},${ev.data.from.y})→(${ev.data.to.x},${ev.data.to.y})`;
    case 'agent_move_failed':
      return `${ev.data.agent} 移失败：${ev.data.reason}`;
    case 'agent_gathered':
      return `${ev.data.agent} 采 ${ev.data.item} ×${ev.data.n} @(${ev.data.from.x},${ev.data.from.y})`;
    case 'agent_gather_failed':
      return `${ev.data.agent} 采失败：${ev.data.reason}`;
    case 'agent_ate':
      return `${ev.data.agent} 食 ${ev.data.item} (饥+${ev.data.hunger_gain} 血+${ev.data.hp_gain})`;
    case 'agent_crafted':
      return `${ev.data.agent} 造 ${ev.data.recipe}`;
    case 'agent_craft_failed':
      return `${ev.data.agent} 造失败：${ev.data.reason}`;
    case 'agent_placed':
      return `${ev.data.agent} 置 ${ev.data.building} @(${ev.data.at.x},${ev.data.at.y})`;
    case 'agent_picked_up':
      return `${ev.data.agent} 拾 ${ev.data.item} ×${ev.data.n}`;
    case 'agent_dropped':
      return `${ev.data.agent} 弃 ${ev.data.item} ×${ev.data.n}`;
    case 'agent_died':
      return `${ev.data.agent} 殁 @(${ev.data.at.x},${ev.data.at.y}) · ${ev.data.cause}`;
    case 'agent_respawned':
      return `${ev.data.agent} 还魂 @(${ev.data.at.x},${ev.data.at.y})`;
    case 'season_changed':
      return `节气转 → ${ev.data.to}`;
    case 'day_started':
      return `第 ${ev.data.day} 日 · 昼`;
    case 'night_started':
      return `第 ${ev.data.day} 日 · 夜`;
  }
}

function pushEvents(tick: number, events: TickEvent[]): void {
  for (const e of events) {
    recentEvents.unshift({ tick, event: e });
  }
  if (recentEvents.length > MAX_EVENTS) {
    recentEvents.length = MAX_EVENTS;
  }
  const list = el<HTMLUListElement>('event-list');
  list.replaceChildren();
  for (const entry of recentEvents) {
    const li = document.createElement('li');
    li.className = `event-row event-${entry.event.kind}`;
    const t = document.createElement('span');
    t.className = 'event-tick';
    t.textContent = `t${entry.tick}`;
    const d = document.createElement('span');
    d.className = 'event-desc';
    d.textContent = describeEvent(entry.event);
    li.appendChild(t);
    li.appendChild(d);
    list.appendChild(li);
  }
  if (recentEvents.length === 0) {
    const li = document.createElement('li');
    li.className = 'empty';
    li.textContent = '风平浪静';
    list.appendChild(li);
  }
}

async function main(): Promise<void> {
  const stage = new WorldStage();
  const stageEl = el<HTMLElement>('stage');
  await stage.mount(stageEl);

  const onMsg = (msg: ServerMsg): void => {
    if (msg.kind === 'snapshot') {
      stage.setGrid(msg.grid_width, msg.grid_height, msg.tiles);
      stage.setEntities(msg.entities);
      stage.setAgents(msg.agents);
      renderClock(msg.tick);
      renderAgents(msg.agents);
    } else {
      const { tick, agents, entities, events } = msg.view;
      stage.setEntities(entities);
      stage.setAgents(agents);
      renderClock(tick);
      renderAgents(agents);
      pushEvents(tick, events);
    }
  };

  connect('ws://127.0.0.1:7777/ws/spectator', onMsg);
}

void main();
