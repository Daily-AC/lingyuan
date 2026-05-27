"""测试 lingyuan_mcp._render_markdown：检验 recent_events 渲染、植物冷却显示
等关键 UI 路径——这些是 agent 实际看到的内容，比 server JSON 更接近用户。"""
from __future__ import annotations

import sys
from pathlib import Path

# 让 import 找到 mcp/lingyuan_mcp.py（pytest 从仓库根跑）
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from lingyuan_mcp import _render_markdown


def _base_obs(**overrides):
    o = {
        "tick": 100,
        "clock": {"season": "chun", "phase": "day", "day": 1, "tick_in_day": 28},
        "self": {
            "id": "ag_me",
            "name": "tester",
            "pos": {"x": 10, "y": 10},
            "status": {"hp": 80, "hunger": 60, "stamina": 70, "warmth": 0, "sanity": 100},
            "state": {"state": "alive"},
            "inventory": [],
        },
        "vision": {"radius": 6, "tiles": []},
        "visible_entities": [],
        "nearby_signs": [],
        "mail": [],
        "recent_events": [],
    }
    o.update(overrides)
    return o


def test_failed_event_renders_with_warning_and_reason():
    obs = _base_obs(recent_events=[
        {
            "kind": "agent_eat_failed",
            "data": {"agent": "ag_me", "reason": "Reed not food"},
        }
    ])
    md = _render_markdown(obs)
    assert "Recent events" in md, "应有 recent events 段头"
    assert "⚠" in md, "失败事件应带 ⚠ 警示符"
    assert "agent_eat_failed" in md
    assert "Reed not food" in md, "reason 必须出现，否则 agent 看不出失败原因"


def test_success_event_renders_without_warning():
    obs = _base_obs(recent_events=[
        {
            "kind": "agent_gathered",
            "data": {"agent": "ag_me", "item": "bamboo", "n": 1, "from": {"x": 9, "y": 10}},
        }
    ])
    md = _render_markdown(obs)
    assert "agent_gathered" in md
    assert "⚠" not in md, "成功事件不该有警示符"
    assert "item=bamboo" in md or "n=1" in md, "应有关键字段摘要"


def test_empty_recent_events_no_section_header():
    obs = _base_obs(recent_events=[])
    md = _render_markdown(obs)
    assert "Recent events" not in md, "无事件时不该打段头"


def test_plant_cooldown_remaining_renders():
    obs = _base_obs(visible_entities=[
        {
            "kind": "plant",
            "pos": {"x": 11, "y": 10},
            "species": "mushroom",
            "available": False,
            "cooldown_remaining": 25,
        }
    ])
    md = _render_markdown(obs)
    assert "mushroom" in md
    assert "25" in md, "冷却剩余 tick 必须暴露"
    assert "冷却" in md


def test_plant_available_no_cooldown_marker():
    obs = _base_obs(visible_entities=[
        {
            "kind": "plant",
            "pos": {"x": 11, "y": 10},
            "species": "red_berry",
            "available": True,
            "cooldown_remaining": None,
        }
    ])
    md = _render_markdown(obs)
    assert "red_berry" in md
    assert "✓" in md, "可采植物应显示 ✓"
    assert "冷却" not in md


def test_plant_unavailable_without_cooldown_field_legacy_fallback():
    # 老版本 server payload 没有 cooldown_remaining 字段，渲染不应该崩
    obs = _base_obs(visible_entities=[
        {
            "kind": "plant",
            "pos": {"x": 11, "y": 10},
            "species": "mushroom",
            "available": False,
            # 缺 cooldown_remaining
        }
    ])
    md = _render_markdown(obs)
    assert "mushroom" in md
    assert "冷却" in md  # 只显示冷却但不带具体数字
