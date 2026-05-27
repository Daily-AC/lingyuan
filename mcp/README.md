# 灵渊 MCP server

把灵渊 REST API 包成 MCP tools，Claude Code / Codex / 其他 MCP 兼容客户端可以直接调用，不必走 shell CLI。

## 暴露的 tools

| tool | 入参 | 作用 |
|------|------|------|
| `lingyuan_world_info` | — | 获取全局时钟 |
| `lingyuan_join` | `name: str` | 注册 agent；token 持久化到 `/tmp/lingyuan-mcp-<pid>.json` |
| `lingyuan_observe` | `format: "json"\|"markdown"` | 拿当前 observation |
| `lingyuan_act` | `action: dict` | 排队动作（schema 见 spec §4.3 / skill 文档） |
| `lingyuan_leave` | — | 主动离场 |

## 装 deps

```bash
cd /Users/e0_7/projects/games/lingyuan
uv venv  # 已存在则跳过
uv pip install mcp httpx
```

## 接 Claude Code

修改 `~/.claude/mcp.json`（或对应版本路径），加：

```json
{
  "mcpServers": {
    "lingyuan": {
      "command": "/Users/e0_7/projects/games/lingyuan/.venv/bin/python",
      "args": ["/Users/e0_7/projects/games/lingyuan/mcp/lingyuan_mcp.py"],
      "env": {
        "LINGYUAN_SERVER": "http://127.0.0.1:7777"
      }
    }
  }
}
```

重启 Claude Code。会看到 `mcp__lingyuan__lingyuan_join` 等工具。

## 用例

Claude Code 内：

```
你: 帮我接入灵渊，名字叫 wukong，先看看周围
Claude: [调 lingyuan_join name="wukong"]
        [调 lingyuan_observe format="markdown"]
        在 (67,40) 入世；周围有 mushroom @(66,39)，blah blah
```

## 单 MCP 实例 = 单 agent

token 文件挂在 server 进程 pid 上。一个 MCP server 进程对应一个 agent 身份。
多 agent 接入 = 多个 MCP server 进程（Claude Code 每个会话默认开一个）。

如果要在同一个 Python 进程里接入多个 agent，自己 fork。

## 自检（命令行）

```bash
source .venv/bin/activate
# 启 lingyuan server
cargo run -p server &

# 直接 import 模块测
python -c "
import sys; sys.path.insert(0, 'mcp')
import lingyuan_mcp as m
print(m.lingyuan_world_info())
print(m.lingyuan_join('test'))
print(m.lingyuan_observe(format='markdown')['markdown'][:300])
print(m.lingyuan_act({'kind':'move','data':{'dir':'north'}}))
print(m.lingyuan_leave())
"
```
