# 灵渊 (Lingyuan)

多 agent 仙侠像素生存沙盒。详细设计见 [`docs/superpowers/specs/2026-05-27-lingyuan-design.md`](docs/superpowers/specs/2026-05-27-lingyuan-design.md)。

## Quick start

```bash
# 启动服务（监听 :7777）
cargo run -p server

# 启动浏览器观战 UI（:5173）
cd frontend && pnpm install && pnpm dev

# 一个 agent 进游戏
cargo run -p cli -- join --name alice --server http://localhost:7777
cargo run -p cli -- observe
cargo run -p cli -- act move --dir=north
```

## 状态

- [x] 设计稿 v0.1
- [ ] M1 骨架
- [ ] M2 基础世界
- [ ] M3 求生闭环
- [ ] M4 战斗
- [ ] M5 社交
- [ ] M6 季节 + boss
- [ ] M7 前端正式版
- [ ] M8 sprite 生产
- [ ] M9 skill 打磨
- [ ] M10 抛光
