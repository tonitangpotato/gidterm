# RFC: Multi-Project Developer Experience (DX)

**Status:** Draft  
**Created:** 2026-02-03  
**Author:** Clawd  

## Motivation

来自 Theo (t3.gg) 的推文指出的痛点：

> "The biggest thing that sucks about working with Coding Agents on multiple projects is keeping track of what's happening. I get the multiple terminal tabs all look the same, multiple browser tabs with different localhosts..."

**核心问题：**
1. 🔍 **可见性差** - 多个terminal tabs，哪个agent完成了？找不到
2. 🔌 **Port冲突** - localhost:3000被谁占了？
3. 🌐 **Browser混乱** - 哪个chrome窗口是哪个项目的？
4. 🧠 **心智负担** - 单项目能记住，多项目完全乱
5. ⏱️ **Context切换** - 开销大于实际coding时间

Theo说他 "almost started to build an OS" 来解决这个问题 - 我们不需要OS级别，但gidterm作为terminal controller已经有了基础，可以成为解决方案。

## Current State

gidterm已经有的能力：
- ✅ Multi-project workspace mode (`--workspace`)
- ✅ 项目隔离（每个项目独立graph）
- ✅ Task DAG scheduling
- ✅ Parallel execution
- ✅ Real-time TUI dashboard

缺失的：
- ❌ 全局项目状态概览
- ❌ Port管理/追踪
- ❌ Agent状态集成
- ❌ 通知聚合
- ❌ 浏览器集成

## Proposed Features

### 1. 🎛️ Unified Dashboard

一眼看到所有项目的关键状态：

```
┌─────────────────────────────────────────────────────────────┐
│  gidterm workspace (3 projects)                    [?] help │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  📁 backend          :3000   🟢 claude-code running        │
│     └─ build [done] → dev [running 2m] → test [pending]    │
│                                                             │
│  📁 frontend         :3001   🔵 waiting for input          │
│     └─ install [done] → build [running] → preview [...]    │
│                                                             │
│  📁 api-gateway      :3002   ⏸️  paused                     │
│     └─ all tasks complete                                   │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│  Recent Events:                                             │
│  • 09:04 [frontend] Agent completed task: "add dark mode"   │
│  • 09:02 [backend] Build succeeded                          │
│  • 09:01 [api-gateway] Agent paused (waiting approval)      │
└─────────────────────────────────────────────────────────────┘
```

**实现要点：**
- 每行显示：项目名、分配的port、agent状态、task pipeline概览
- 底部显示最近事件，highlight需要注意的
- 颜色编码：🟢运行中 🔵需输入 🟡警告 🔴错误 ⏸️暂停

### 2. 🔌 Port Management

自动管理开发服务器端口，避免冲突：

```yaml
# .gid/graph.yml 中的port配置
metadata:
  project: "backend"
  port: auto          # gidterm自动分配
  # 或者
  port: 3000          # 首选port
  port_fallback: true # 冲突时自动+1
```

**功能：**
- 自动扫描 `3000-3999` 范围找可用port
- 维护全局 port registry（`~/.gidterm/ports.json`）
- 启动时注入 `$PORT` 环境变量
- Port冲突检测和自动解决
- `gidterm ports` 命令查看当前分配

```bash
$ gidterm ports
PORT    PROJECT         PROCESS         STATUS
3000    backend         npm run dev     🟢 active
3001    frontend        vite            🟢 active  
3002    api-gateway     -               ⏸️ reserved
```

### 3. 🤖 Agent Integration

与coding agent深度集成：

**支持的agents：**
- Claude Code (`claude`)
- Codex CLI (`codex`)
- OpenCode (`opencode`)
- Pi Coding Agent

**集成方式：**
```yaml
# .gid/graph.yml
tasks:
  implement-feature:
    agent: claude          # 指定使用哪个agent
    prompt: "Implement user authentication"
    status: pending
```

**状态追踪：**
- 检测agent进程是否运行
- 解析agent输出判断状态（running/waiting/completed/error）
- Agent完成时触发通知

### 4. 🔔 Notification Aggregation

统一通知中心：

```
┌────────────────────────────────────────┐
│ 🔔 gidterm                             │
├────────────────────────────────────────┤
│ [backend] Agent completed!             │
│ Task: implement-auth                   │
│ Duration: 4m 32s                       │
│                                        │
│ [View] [Approve] [Next Task]          │
└────────────────────────────────────────┘
```

**通知渠道：**
- macOS Notification Center（默认）
- Terminal bell
- (可选) Webhook to Telegram/Discord
- (可选) 声音提示

**配置：**
```yaml
# ~/.gidterm/config.yml
notifications:
  on_complete: true
  on_error: true
  on_waiting: true      # agent等待输入时
  sound: true
  channels:
    - system            # OS notification
    - telegram          # 可选
```

### 5. ⚡ Quick Switch

快速在项目间切换：

```bash
# CLI方式
gidterm switch backend      # 聚焦到backend项目
gidterm focus frontend      # 同上

# TUI中
# 按 1/2/3 数字键快速切换
# 或 / 搜索项目名
```

**切换时的动作：**
- 将该项目的terminal带到前台
- 打开相关的browser tabs（如果有集成）
- 更新TUI焦点

### 6. 🌐 Browser Integration (Phase 2)

通过Chrome Extension实现tab分组：

**功能：**
- 按项目自动分组tabs
- 识别 `localhost:PORT` 并关联到项目
- 一键打开项目的所有相关URLs
- 关闭项目时可选关闭相关tabs

**实现路径：**
1. Chrome Extension监听tab创建
2. Extension与gidterm通过WebSocket通信
3. gidterm发送port-project映射
4. Extension自动给tabs打标签/分组

```
Chrome Tab Groups:
├── 📁 backend (localhost:3000)
│   ├── App - localhost:3000
│   └── API Docs - localhost:3000/docs
├── 📁 frontend (localhost:3001)
│   └── Vite - localhost:3001
└── Other tabs...
```

## Implementation Plan

### Phase 1: Core DX (2-3 weeks)
- [ ] Unified dashboard redesign
- [ ] Port management system
- [ ] Basic notifications (system)
- [ ] Quick switch (keyboard shortcuts)

### Phase 2: Agent Integration (2 weeks)
- [ ] Agent process detection
- [ ] Agent status parsing
- [ ] Agent task definition in graph

### Phase 3: Browser Integration (2-3 weeks)
- [ ] Chrome Extension scaffold
- [ ] WebSocket bridge
- [ ] Tab grouping logic
- [ ] URL-to-project mapping

### Phase 4: Polish (1 week)
- [ ] Configuration system
- [ ] Documentation
- [ ] Demo video

## Open Questions

1. **Port persistence** - 每次启动用同样的port还是fresh分配？
   - 建议：持久化，但检测冲突时重新分配

2. **Agent detection** - 如何判断agent状态？
   - 解析stdout关键词？检测进程？Agent API？
   - 建议：先做进程检测 + stdout关键词，后续可以加API

3. **Cross-platform** - 是否支持Linux/Windows？
   - macOS优先，Linux次之，Windows低优先级

4. **与其他工具的关系** - tmux/Warp/iTerm？
   - gidterm是独立TUI，不依赖也不替代这些工具
   - 可以在tmux里运行gidterm

## Alternatives Considered

1. **VS Code Extension** - 更深的IDE集成
   - 缺点：绑定VS Code，不够通用

2. **Electron App** - 图形界面
   - 缺点：重，开发成本高

3. **tmux wrapper** - 包装tmux
   - 缺点：tmux学习曲线，配置复杂

选择TUI的原因：轻量、跨终端、符合开发者习惯

## Success Metrics

- 项目切换时间 < 2秒
- Port冲突率 → 0
- "找agent"的时间 → 0（直接看dashboard）
- 用户不再需要肉眼扫描多个terminal tabs

## References

- [Theo's tweet thread](https://twitter.com/t3dotgg/...) 
- [gidterm design.md](./design.md)
- [mprocs](https://github.com/pvolok/mprocs) - 多进程TUI参考
- [Chrome Tab Groups API](https://developer.chrome.com/docs/extensions/reference/tabGroups/)
