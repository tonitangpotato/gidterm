# gidterm + gid MCP Integration Plan

## Goal

让 gidterm 能够：
1. 读取 gid 维护的项目图（`.gid/graph.yml`）
2. 跨 session 跟踪任务历史
3. 语义理解不同类型的任务
4. 多项目管理

---

## Current State

**gidterm 现在**:
- ✅ 读取独立的 YAML 文件
- ✅ 实时跟踪单次运行
- ✅ 基础任务调度
- ❌ 不集成 gid
- ❌ 不持久化历史

**gid MCP 提供**:
- 项目图结构（nodes, tasks, dependencies）
- 语义标记（node types, layers）
- 项目元数据

---

## Integration Architecture

```
gid MCP (Graph Source)
    ↓
.gid/graph.yml
    ↓
gidterm (Executor)
    ↓
.gidterm/sessions/ (History)
```

---

## Phase 1: Read gid Graphs

### Changes needed

**1. Add gid graph loader**

```rust
// src/core/graph.rs

impl Graph {
    /// Load from gid project directory
    pub fn from_gid_project(project_dir: &Path) -> Result<Self> {
        let gid_path = project_dir.join(".gid/graph.yml");
        Self::from_file(&gid_path)
    }
    
    /// Auto-detect graph location
    pub fn auto_load() -> Result<Self> {
        // Try .gid/graph.yml first
        if Path::new(".gid/graph.yml").exists() {
            return Self::from_file(".gid/graph.yml");
        }
        
        // Fall back to gidterm.yml or other configs
        if Path::new("gidterm.yml").exists() {
            return Self::from_file("gidterm.yml");
        }
        
        Err(anyhow::anyhow!("No graph file found"))
    }
}
```

**2. Update CLI to support gid projects**

```bash
# Auto-detect
gidterm

# Explicit gid project
gidterm --gid-project .

# Legacy YAML file
gidterm custom.yml
```

---

## Phase 2: Session Persistence

### Data Structure

```rust
// src/session.rs

pub struct Session {
    pub id: String,
    pub project: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub tasks: HashMap<String, TaskHistory>,
}

pub struct TaskHistory {
    pub task_id: String,
    pub runs: Vec<TaskRun>,
}

pub struct TaskRun {
    pub started: DateTime<Utc>,
    pub ended: Option<DateTime<Utc>>,
    pub status: TaskStatus,
    pub output: Vec<String>,
    pub exit_code: Option<i32>,
}
```

### Storage

```
.gidterm/
├── sessions/
│   ├── 2026-01-31-16-57-28.json      # This run
│   ├── 2026-01-31-15-23-10.json      # Previous run
│   └── latest.json -> 2026-01-31-16-57-28.json
└── config.yml                         # gidterm settings
```

### Features

- **Resume session**: `gidterm --resume`
- **View history**: `gidterm history`
- **Clean old sessions**: `gidterm clean --older-than 7d`

---

## Phase 3: Semantic Understanding

### Task Type Handlers

```rust
// src/semantic/handlers/

// Build tasks
pub struct BuildHandler;
impl OutputParser for BuildHandler {
    fn parse(&self, output: &str) -> TaskMetrics {
        // Detect: "Building 45/100 modules"
        // Extract progress: 45%
    }
}

// Test tasks
pub struct TestHandler;
impl OutputParser for TestHandler {
    fn parse(&self, output: &str) -> TaskMetrics {
        // Detect: "✓ 42 tests passed, 3 failed"
        // Extract: passed=42, failed=3
    }
}

// Service tasks
pub struct ServiceHandler;
impl OutputParser for ServiceHandler {
    fn parse(&self, output: &str) -> TaskMetrics {
        // Detect: "Server listening on :3000"
        // Status: running
        
        // Detect errors/crashes
        // Status: failed
    }
}
```

### Enhanced TUI with Semantics

```
╔═══════════════════════════════════════════════════╗
║ 📊 AgentVerse | Running: 3 | Tests: 42/45 ✓     ║
╠═══════════════════════════════════════════════════╣
║                                                   ║
║ ⚙ backend-build [in-progress] 🟡 (234L) 67% ██▌ ║ ← Progress bar
║ ✓ backend-test [done] ✅ 42/45 tests passed      ║ ← Test results
║ ⚙ backend-dev [running] 🟢 UP 5m               ║ ← Service uptime
║                                                   ║
╚═══════════════════════════════════════════════════╝
```

---

## Phase 4: Multi-Project Management

### Project Registry

```
~/.gidterm/
└── projects.yml

projects:
  - name: agentverse
    path: ~/clawd/agentverse
    last_run: 2026-01-31T16:57:28Z
    
  - name: bot-town
    path: ~/clawd/bot-town
    last_run: 2026-01-30T10:23:45Z
```

### Commands

```bash
# List projects
gidterm projects

# Switch project
gidterm -p agentverse

# Run specific project
gidterm run agentverse
```

---

## Implementation Priority

### P0 (Essential for AgentVerse dev)
1. ✅ Real-time TUI (Done!)
2. 🚧 Read from `.gid/graph.yml`
3. 🚧 Basic session persistence

### P1 (Nice to have)
4. Semantic task type handlers
5. Multi-project management

### P2 (Future)
6. Full gid MCP bidirectional sync
7. Remote execution
8. Team collaboration

---

## Implementation Status

### ✅ Phase 1: Read gid Graphs (DONE)

**Completed**:
- ✅ Added `Graph::from_gid_project()` method
- ✅ Added `Graph::auto_load()` for smart detection
- ✅ Updated CLI to auto-detect graph files
- ✅ Priority: `.gid/graph.yml` → `gidterm.yml`

### ✅ Phase 2: Session Persistence (DONE)

**Completed**:
- ✅ Created `src/session.rs` module
- ✅ Implemented session storage in `.gidterm/sessions/`
- ✅ Integrated session tracking into App
- ✅ Auto-save session on task start/completion
- ✅ Session ends on app quit

**What's tracked**:
- Task start/end times
- Task status (Running, Done, Failed)
- Exit codes
- Output lines per task

### ⏳ Phase 3: Semantic Understanding (LATER)
### ⏳ Phase 4: Multi-Project Management (LATER)

---

## Current Sprint (2-3 hours)

**Goal**: Make gidterm work seamlessly with gid projects and persist task history

**Timeline**:
1. **30 min** - gid graph loading
2. **60 min** - session persistence
3. **30 min** - CLI updates
4. **30 min** - testing

**Then**: Move to AgentVerse development!
