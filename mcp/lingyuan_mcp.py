#!/usr/bin/env python3
"""
灵渊 MCP server。把 lingyuan REST API 暴露为 MCP tools，让 Claude Code 等 agent
不必走 shell CLI 就能 join/observe/act。

使用：
  在 Claude Code config 加：
    {
      "mcpServers": {
        "lingyuan": {
          "command": "/Users/e0_7/projects/games/lingyuan/.venv/bin/python",
          "args": ["/Users/e0_7/projects/games/lingyuan/mcp/lingyuan_mcp.py"],
          "env": { "LINGYUAN_SERVER": "http://127.0.0.1:7777" }
        }
      }
    }

提供 tools：
  - lingyuan_join(name)           注册 agent，token 存到 /tmp/lingyuan-mcp-<pid>.json
  - lingyuan_observe()            返回当前 observation（markdown 或 json）
  - lingyuan_act(action_json)     发动作（已有 join 状态）
  - lingyuan_world_info()         全局时钟 + agent 列表
  - lingyuan_leave()              主动离场 + 删 token

设计选择：
  - 一个 MCP server 实例对应一个 agent 身份（token 文件挂在 pid 上）
  - 多 agent 接入 = 多个 MCP server 实例（Claude Code 每个会话开一个）
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path
from typing import Any

import httpx
from mcp.server.fastmcp import FastMCP

SERVER_URL = os.environ.get("LINGYUAN_SERVER", "http://127.0.0.1:7777")
TOKEN_PATH = Path(os.environ.get(
    "LINGYUAN_MCP_TOKEN",
    f"/tmp/lingyuan-mcp-{os.getpid()}.json",
))

app = FastMCP("lingyuan")


def _load_token() -> dict[str, str] | None:
    if not TOKEN_PATH.exists():
        return None
    try:
        return json.loads(TOKEN_PATH.read_text())
    except Exception:
        return None


def _save_token(t: dict[str, str]) -> None:
    TOKEN_PATH.write_text(json.dumps(t, indent=2))


def _client() -> httpx.Client:
    # 显式 trust_env=False 绕开系统 proxy（本地 server 必须直连）
    return httpx.Client(base_url=SERVER_URL, trust_env=False, timeout=10.0)


def _auth_headers(t: dict[str, str]) -> dict[str, str]:
    return {
        "Authorization": f"Bearer {t['token']}",
        "X-Agent-Id": t["agent_id"],
        "Content-Type": "application/json",
    }


@app.tool()
def lingyuan_join(name: str) -> dict[str, Any]:
    """注册一个 agent 进入灵渊世界。name 必须 1~32 字符且未被占用。
    成功后 token 写入 /tmp，后续 observe/act 自动用此 token。"""
    if not name or len(name) > 32:
        return {"ok": False, "error": "name must be 1..=32 chars"}
    existing = _load_token()
    if existing:
        return {
            "ok": False,
            "error": f"already joined as {existing['name']} (id {existing['agent_id']}). 先 lingyuan_leave 再 join",
        }
    with _client() as c:
        r = c.post("/api/v1/join", json={"name": name})
        if r.status_code != 200:
            return {"ok": False, "error": f"HTTP {r.status_code}: {r.text}"}
        data = r.json()
    t = {
        "agent_id": data["agent_id"],
        "token": data["token"],
        "name": name,
        "server": SERVER_URL,
    }
    _save_token(t)
    return {
        "ok": True,
        "agent_id": data["agent_id"],
        "spawn_at": data["spawn_at"],
        "tick": data["tick"],
        "name": name,
    }


@app.tool()
def lingyuan_observe(format: str = "json") -> dict[str, Any]:
    """获取当前观察（自身状态 + 视野 tiles + 视野内实体 + 信件 + 路牌）。
    format='json' 返回结构化数据；format='markdown' 返回人类易读文本。"""
    t = _load_token()
    if t is None:
        return {"ok": False, "error": "未注册，先 lingyuan_join"}
    with _client() as c:
        r = c.get("/api/v1/observe", headers=_auth_headers(t))
        if r.status_code != 200:
            return {"ok": False, "error": f"HTTP {r.status_code}: {r.text}"}
        obs = r.json()
    if format == "markdown":
        return {"ok": True, "markdown": _render_markdown(obs)}
    return {"ok": True, "observation": obs}


@app.tool()
def lingyuan_act(action: dict[str, Any]) -> dict[str, Any]:
    """发动作。action 是 {kind: <verb>, data: {...}} 形态。

    支持的 kind（完整 craft recipe 列表/inputs 见 lingyuan_world_info）：
      - move: data={dir: "north"|"south"|"east"|"west"}
      - wait: data={}
      - gather: data={target: {x, y}}      # manhattan ≤ 1
      - eat: data={item: "<snake_case_item>"}
      - craft: data={recipe: "<recipe_id>"}
      - place: data={item: "<kit>", pos: {x, y}}     # manhattan ≤ 1
      - pick_up: data={pos: {x, y}}                  # manhattan ≤ 1
      - drop: data={item: "<item>", n: <int>}
      - attack: data={target: {target_kind: "agent"|"creature", target_id: <id>}}  # manhattan ≤ 1
      - write_sign: data={pos: {x, y}, text: "<≤200 字>"}
      - send_mail: data={to: "<agent_name>", text: "<≤500 字>"}

    成功返回 {accepted: true, accepted_at_tick, will_resolve_at_tick, queue_depth}。
    同 tick 内若已有 action 排队，返回 HTTP 409 + {accepted: false,
    reason: "already_queued", existing_action, will_resolve_at_tick}——本工具
    把它转成 {ok: false, already_queued: true, ...}。规则：等到
    will_resolve_at_tick 落地后再发下一个动作；不要在同 tick 内连发。"""
    t = _load_token()
    if t is None:
        return {"ok": False, "error": "未注册，先 lingyuan_join"}
    with _client() as c:
        r = c.post("/api/v1/act", json=action, headers=_auth_headers(t))
        if r.status_code == 409:
            return {"ok": False, "already_queued": True, **r.json()}
        if r.status_code != 200:
            return {"ok": False, "error": f"HTTP {r.status_code}: {r.text}"}
        return {"ok": True, **r.json()}


@app.tool()
def lingyuan_world_info() -> dict[str, Any]:
    """全局世界信息。不需要 token。

    返回：
      - clock: {tick, day, season, phase, tick_in_day}
      - constants: {vision_radius, interaction_range, inventory_slots,
        max_hp/hunger/stamina, *_period_ticks, weapon_damage[...] ...}
      - recipes: [{id, inputs:[{item,n}], output:{item,n}, station}] 全 9 条
      - items: [{id, name_zh, is_food, nutrition?, stack_size}] 全 19 个

    必看：开局 craft 前先看 recipes，攻击前看 weapon_damage 选武器，
    采集/攻击/放置 距离都受 constants.interaction_range（=1 manhattan）限。"""
    with _client() as c:
        r = c.get("/api/v1/world/info")
        if r.status_code != 200:
            return {"ok": False, "error": f"HTTP {r.status_code}: {r.text}"}
        return {"ok": True, **r.json()}


@app.tool()
def lingyuan_leave() -> dict[str, Any]:
    """主动离场 + 清除本地 token 文件。"""
    t = _load_token()
    if t is None:
        return {"ok": True, "note": "未注册，无需 leave"}
    with _client() as c:
        c.post("/api/v1/leave", headers=_auth_headers(t))
    TOKEN_PATH.unlink(missing_ok=True)
    return {"ok": True, "left": t["name"]}


def _render_markdown(obs: dict[str, Any]) -> str:
    name = obs.get("self", {}).get("name", "?")
    tick = obs.get("tick", 0)
    clk = obs.get("clock", {})
    status = obs.get("self", {}).get("status", {})
    parts = [
        f"## You are {name} — tick {tick}",
        f"**Clock**: {clk.get('season','?')}·{clk.get('phase','?')} 第 {clk.get('day',0)} 日 刻 {clk.get('tick_in_day',0)}",
        f"**Status**: HP {status.get('hp')}/100 · 饥 {status.get('hunger')}/100 · 力 {status.get('stamina')}/100 · 温 {status.get('warmth')} · 灵识 {status.get('sanity')}",
        "",
    ]
    pos = obs.get("self", {}).get("pos", {})
    raw_state = obs.get("self", {}).get("state", {})
    state_label = raw_state.get("state") if isinstance(raw_state, dict) else str(raw_state)
    parts.append(f"**You at** ({pos.get('x')},{pos.get('y')})  state={state_label}")
    inv = obs.get("self", {}).get("inventory", [])
    if inv:
        parts.append("**Inventory**: " + ", ".join(f"{i['item']}×{i['n']}" for i in inv))
    else:
        parts.append("**Inventory**: 空袋")
    parts.append("")
    parts.append("**Visible entities**:")
    for e in obs.get("visible_entities", []):
        kind = e.get("kind")
        if kind == "agent":
            parts.append(f"  - agent {e['name']} @({e['pos']['x']},{e['pos']['y']}) hp {e['hp']}")
        elif kind == "plant":
            avail = "✓" if e.get("available") else "✗冷却"
            parts.append(f"  - {e['species']} @({e['pos']['x']},{e['pos']['y']}) {avail}")
        elif kind == "item_drop":
            parts.append(f"  - drop {e['item']}×{e['n']} @({e['pos']['x']},{e['pos']['y']}) ttl {e['expires_in']}")
        elif kind == "building":
            parts.append(f"  - building {e['subkind']} @({e['pos']['x']},{e['pos']['y']}) by {e['owner']}")
        elif kind == "creature":
            h = "敌" if e.get("hostile") else "中"
            parts.append(f"  - creature #{e['id']} {e['species']} {h} @({e['pos']['x']},{e['pos']['y']}) hp{e['hp']}")
    parts.append("")
    signs = obs.get("nearby_signs", [])
    if signs:
        parts.append("**Signs nearby**:")
        for s in signs:
            parts.append(f"  - ({s['pos']['x']},{s['pos']['y']}) \"{s['text']}\" — {s.get('author') or 'anon'}")
        parts.append("")
    mail = obs.get("mail", [])
    if mail:
        parts.append("**Mail**:")
        for m in mail:
            parts.append(f"  - {m['from']} (t{m['received_at_tick']}): {m['text']}")
    return "\n".join(parts)


if __name__ == "__main__":
    app.run()
