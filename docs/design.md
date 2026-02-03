# GidTerm: Graph-Driven Semantic Terminal Controller

*概念设计：将 gid 的 project/task graph 与 terminal control panel 结合*

---

## 🎯 核心概念

传统终端控制器只是扁平的进程列表：
```
Window 1 → Window 2 → Window 3
```

GidTerm 的愿景：
```
        Project Graph
             │
    ┌────────┼────────┐
    ▼        ▼        ▼
  Task 1  Task 2  Task 3
    │        │        │
    ▼        ▼        ▼
Terminal Terminal Terminal
```

---

## 🧠 三层架构

### 层 1: Graph 层（语义）
```rust
struct ProjectGraph {
    nodes: Vec<ProjectNode>,
    edges: Vec<Dependency>,
}

struct TaskGraph {
    tasks: Vec<Task>,
    dependencies: HashMap<TaskId, Vec<TaskId>>,
}

struct Task {
    id: TaskId,
    name: String,
    command: String,
    terminal_id: Option<TerminalId>,
    status: TaskStatus,
    progress: f32,
    dependencies: Vec<TaskId>,
}

enum TaskStatus {
    Waiting,      // 等待依赖完成
    Ready,        // 可以开始
    Running,      // 正在运行
    Done,         // 完成
    Failed,       // 失败
}
```

### 层 2: 语义理解层（智能）
```rust
struct SemanticParser {
    // 解析终端输出，提取语义信息
    parsers: HashMap<TaskType, Box<dyn OutputParser>>,
}

trait OutputParser {
    fn parse(&self, output: &str) -> TaskMetrics;
    fn extract_progress(&self, output: &str) -> Option<f32>;
    fn detect_errors(&self, output: &str) -> Vec<Error>;
}

// 例如：ML 训练任务解析器
struct MLTrainingParser;
impl OutputParser for MLTrainingParser {
    fn parse(&self, output: &str) -> TaskMetrics {
        // 解析 "Epoch 45/100 | Loss: 0.234"
        TaskMetrics {
            progress: 0.45,
            metrics: {
                "loss": 0.234,
                "epoch": 45,
            }
        }
    }
}
```

### 层 3: Terminal 控制层（执行）
```rust
struct TerminalController {
    terminals: HashMap<TerminalId, ManagedWindow>,
    task_bindings: HashMap<TaskId, TerminalId>,
}

impl TerminalController {
    // 根据 task graph 自动启动任务
    fn start_task(&mut self, task: &Task) {
        let terminal_id = self.create_terminal();
        terminal_id.send_command(&task.command);
        self.task_bindings.insert(task.id, terminal_id);
    }
    
    // 从输出更新任务状态
    fn update_task_from_output(&mut self, task_id: TaskId) {
        let terminal = self.get_terminal(task_id);
        let output = terminal.get_recent_output();
        
        let parser = self.get_parser_for_task(task_id);
        let metrics = parser.parse(&output);
        
        // 更新 task graph
        self.task_graph.update_progress(task_id, metrics.progress);
    }
}
```

---

## 🎨 UI 设计：多视图切换

### 视图 1: Graph View（全局视角）
```
╔════════════════════════════════════════════════╗
║         Project: ML Training Pipeline          ║
╠════════════════════════════════════════════════╣
║                                                ║
║   Data Prep ──────┐                           ║
║     ✓ Download    │                           ║
║     ✓ Clean       ├──→ Model Training         ║
║     ⚙ Features    │      ⚙ ResNet (45%)      ║
║                   │      □ EfficientNet        ║
║                   │      □ ViT                 ║
║                   │           │                ║
║                   └───────────┼──→ Evaluation  ║
║                               │      □ Tests   ║
║                               └────→ □ Report  ║
║                                                ║
║ Legend: ✓ Done  ⚙ Running  □ Waiting          ║
╚════════════════════════════════════════════════╝
```

### 视图 2: Task Dashboard（任务级别）
```
╔════════════════════════════════════════════════╗
║              Active Tasks                      ║
╠════════════════════════════════════════════════╣
║                                                ║
║  [1] ResNet Training     █████████░  89%  ⚙   ║
║      Epoch 178/200                             ║
║      Loss: 0.234 | Acc: 0.876                  ║
║                                        [Focus] ║
║                                                ║
║  [2] Feature Gen         ████████░░  78%  ⚙   ║
║      Processing batch 7800/10000               ║
║                                        [Focus] ║
║                                                ║
║  [3] Data Download       ████████░░  85%  ⚙   ║
║      1.2GB / 1.4GB                             ║
║                                        [Focus] ║
╚════════════════════════════════════════════════╝
```

### 视图 3: Terminal View（执行细节）
```
╔════════════════════════════════════════════════╗
║  Task: ResNet Training (Window 1)              ║
╠════════════════════════════════════════════════╣
║                                                ║
║  > python train.py --model resnet50            ║
║  Loading dataset...                            ║
║  Epoch 178/200                                 ║
║  Loss: 0.234 | Acc: 0.876 | LR: 0.001         ║
║  [=========>          ] 45%                    ║
║                                                ║
╠════════════════════════════════════════════════╣
║ Task Actions:                                  ║
║  [s] Save checkpoint  [p] Pause  [k] Kill      ║
║  [l] Adjust LR        [v] Visualize metrics    ║
╚════════════════════════════════════════════════╝
```

---

## 💡 核心 Semantic Features

### Feature 1: 任务类型感知 🏷️

```yaml
# gid 项目配置
tasks:
  train_resnet:
    type: ml_training  # 任务类型
    command: python train.py
    parser: ml_parser  # 自动使用对应解析器
    
  build_frontend:
    type: build_task
    command: npm run build
    parser: build_parser
    
  run_tests:
    type: test_suite
    command: pytest
    parser: pytest_parser
```

**自动配置：**
```rust
match task.type {
    TaskType::MLTraining => {
        // 提供 ML 专用控制
        actions: ["Save Checkpoint", "Adjust LR", "Plot Loss"]
        parser: MLParser
    }
    TaskType::BuildTask => {
        actions: ["Clean Build", "Skip Tests"]
        parser: BuildOutputParser
    }
    TaskType::WebServer => {
        actions: ["Restart", "Hot Reload", "Check Health"]
        parser: ServerLogParser
    }
}
```

---

### Feature 2: 智能命令翻译 🧠

```
用户在 UI 点击：[Save Checkpoint]
  ↓
系统查找 task 定义：
  task.commands.save_checkpoint = "model.save('ckpt.pth')"
  ↓
自动发送到 terminal：
  > model.save('ckpt.pth')
```

**配置示例：**
```yaml
task: train_model
  semantic_commands:
    save_checkpoint: "model.save('checkpoint.pth')"
    adjust_lr: "optimizer.param_groups[0]['lr'] = {value}"
    early_stop: "trainer.should_stop = True"
    
  # 或者用脚本
  control_script: ./model_control.py
```

**实现：**
```rust
struct SemanticCommand {
    label: String,         // UI 上显示的
    template: String,      // 实际命令模板
    params: Vec<Param>,    // 需要的参数
}

// 用户点击 "Adjust LR"
fn execute_semantic_command(cmd: &SemanticCommand, params: HashMap<String, Value>) {
    let actual_cmd = cmd.template
        .replace("{value}", &params["value"].to_string());
    
    terminal.send_command(&actual_cmd);
}
```

---

### Feature 3: 依赖关系自动调度 📊

```yaml
# Task Graph
tasks:
  download_data:
    command: wget https://...
    
  preprocess:
    command: python preprocess.py
    depends_on: [download_data]
    
  train_model:
    command: python train.py
    depends_on: [preprocess]
    
  evaluate:
    command: python eval.py
    depends_on: [train_model]
```

**自动执行：**
```rust
impl TaskGraph {
    fn auto_schedule(&mut self) {
        loop {
            // 找到所有依赖已满足的任务
            let ready_tasks = self.get_ready_tasks();
            
            for task in ready_tasks {
                // 自动启动
                self.controller.start_task(task);
            }
            
            // 检查完成的任务
            let completed = self.get_completed_tasks();
            for task in completed {
                // 标记依赖它的任务为 ready
                self.mark_dependents_ready(task);
            }
            
            if self.all_done() { break; }
            sleep(1s);
        }
    }
}
```

**UI 实时更新：**
```
初始状态：
[1] Download   □ Waiting
[2] Preprocess □ Waiting (depends: Download)
[3] Train      □ Waiting (depends: Preprocess)
[4] Evaluate   □ Waiting (depends: Train)

运行中：
[1] Download   ✓ Done
[2] Preprocess ⚙ Running (45%)
[3] Train      □ Ready (can start)
[4] Evaluate   □ Waiting (depends: Train)
```

---

### Feature 4: 结构化指标提取 📈

```rust
struct TaskMetrics {
    progress: f32,
    custom_metrics: HashMap<String, MetricValue>,
    timestamps: Vec<Timestamp>,
}

// ML 训练示例
impl MLParser {
    fn parse(&self, output: &str) -> TaskMetrics {
        // "Epoch 45/100 | Loss: 0.234 | Acc: 0.876"
        TaskMetrics {
            progress: 0.45,
            custom_metrics: {
                "epoch": 45,
                "loss": 0.234,
                "accuracy": 0.876,
            }
        }
    }
}
```

**可视化：**
```
╔════════════════════════════════════════╗
║  ResNet Training Metrics               ║
╠════════════════════════════════════════╣
║                                        ║
║  Loss:                                 ║
║   0.5 ┤                                ║
║   0.4 ┤      ╲                         ║
║   0.3 ┤       ╲___                     ║
║   0.2 ┤           ╲_____ ← current     ║
║   0.1 ┤                                ║
║       └─────────────────────           ║
║        0    50   100  150  200 epochs  ║
║                                        ║
║  Accuracy: 87.6% ↑                     ║
║  Learning Rate: 0.001                  ║
╚════════════════════════════════════════╝
```

---

### Feature 5: 智能建议系统 🤖

```rust
struct SmartAdvisor {
    rules: Vec<Rule>,
}

struct Rule {
    condition: Box<dyn Fn(&TaskMetrics) -> bool>,
    suggestion: String,
    auto_action: Option<Action>,
}

// 示例规则
let rules = vec![
    Rule {
        condition: |m| m.get("loss") > 1.0 && m.progress > 0.2,
        suggestion: "Loss 还很高，考虑降低学习率？",
        auto_action: Some(Action::SuggestLR(0.0001)),
    },
    
    Rule {
        condition: |m| m.get("loss").is_nan(),
        suggestion: "Loss 变成 NaN，训练可能爆了！",
        auto_action: Some(Action::PauseTask),
    },
];
```

**UI 提示：**
```
╔════════════════════════════════════════╗
║  ⚠️  Smart Suggestion                  ║
╠════════════════════════════════════════╣
║  Loss hasn't decreased in 20 epochs.   ║
║                                        ║
║  Suggestions:                          ║
║   • Reduce learning rate to 0.0001     ║
║   • Increase batch size                ║
║   • Check data quality                 ║
║                                        ║
║  [Apply] [Dismiss]                     ║
╚════════════════════════════════════════╝
```

---

### Feature 6: 上下文感知控制 🎮

```rust
impl TaskController {
    // 根据任务状态提供不同的操作
    fn get_available_actions(&self, task: &Task) -> Vec<Action> {
        match task.status {
            TaskStatus::Running => vec![
                Action::Pause,
                Action::SaveCheckpoint,  // ML 特有
                Action::AdjustParams,
                Action::Kill,
            ],
            
            TaskStatus::Paused => vec![
                Action::Resume,
                Action::Kill,
            ],
            
            TaskStatus::Done => vec![
                Action::ViewResults,
                Action::Restart,
                Action::ExportMetrics,
            ],
            
            TaskStatus::Failed => vec![
                Action::ViewLogs,
                Action::Retry,
                Action::Debug,
            ],
        }
    }
}
```

---

### Feature 7: 多粒度进度追踪 📊

```
Project 级别：
ML Pipeline: [=========>      ] 65%
  ├─ Data Prep: [==============] 100% ✓
  ├─ Training:  [========>     ] 60% ⚙
  └─ Eval:      [              ] 0% □

Task 级别：
Train ResNet: [=========>      ] 89%
  ├─ Init:      [==============] 100% ✓
  ├─ Load Data: [==============] 100% ✓
  ├─ Training:  [=========>    ] 89% ⚙
  │   Epoch 178/200
  └─ Validate:  [              ] 0% □

Sub-task 级别：
Training Loop: [=========>     ] 89%
  Current Batch: 1234/1400
  ETA: 23 minutes
```

---

## 🚀 完整工作流示例

### 场景：ML 训练 Pipeline

```yaml
# project.gid.yaml
project: ML-Training-Pipeline

tasks:
  download_data:
    type: download
    command: wget https://dataset.com/data.zip
    
  preprocess:
    type: data_processing
    command: python preprocess.py
    depends_on: [download_data]
    parser: progress_bar_parser
    
  train_resnet:
    type: ml_training
    command: python train.py --model resnet50
    depends_on: [preprocess]
    parser: ml_training_parser
    semantic_commands:
      save: "trainer.save_checkpoint()"
      adjust_lr: "trainer.set_lr({value})"
      early_stop: "trainer.stop()"
      
  train_efficientnet:
    type: ml_training
    command: python train.py --model efficientnet
    depends_on: [preprocess]
    
  evaluate:
    type: evaluation
    command: python eval.py
    depends_on: [train_resnet, train_efficientnet]
```

**运行：**
```bash
$ gidterm project.gid.yaml

# 自动：
# 1. 解析 task graph
# 2. 启动第一个任务 (download_data)
# 3. 监控进度
# 4. 完成后自动启动 preprocess
# 5. 完成后并行启动两个训练任务
# 6. 全部完成后启动 evaluate
```

**实时 UI：**
```
╔════════════════════════════════════════════════╗
║  [Graph] [Tasks] [Terminal]   Project: 65%    ║
╠════════════════════════════════════════════════╣
║                                                ║
║  Active Tasks:                                 ║
║                                                ║
║  [1] Train ResNet       █████████░  89%  ⚙    ║
║      Epoch 178/200 | Loss: 0.234               ║
║      💡 Loss stable, looking good!             ║
║      [Save] [Adjust LR] [Stop] [Focus]         ║
║                                                ║
║  [2] Train EfficientNet ████████░░  76%  ⚙    ║
║      Epoch 152/200 | Loss: 0.189               ║
║      ⚠️  Loss hasn't improved in 10 epochs     ║
║      [Save] [Adjust LR] [Stop] [Focus]         ║
║                                                ║
║  Completed:                                    ║
║  [✓] Download Data                             ║
║  [✓] Preprocess                                ║
║                                                ║
║  Waiting:                                      ║
║  [□] Evaluate (depends: 1, 2)                  ║
║                                                ║
╠════════════════════════════════════════════════╣
║ > _                                            ║
╚════════════════════════════════════════════════╝
```

---

## 🎯 技术栈

```rust
// 核心架构
gidterm/
├── graph/           # Graph 管理
│   ├── project.rs   # Project graph
│   ├── task.rs      # Task graph
│   └── scheduler.rs # 依赖调度
│
├── semantic/        # 语义层
│   ├── parsers/     # 各种解析器
│   │   ├── ml.rs
│   │   ├── build.rs
│   │   └── generic.rs
│   ├── commands.rs  # 语义命令
│   └── advisor.rs   # 智能建议
│
├── terminal/        # 执行层
│   ├── pty.rs       # PTY 管理
│   ├── controller.rs
│   └── output.rs
│
└── ui/              # UI 层
    ├── graph_view.rs
    ├── task_view.rs
    └── terminal_view.rs
```

---

## 🔍 现有类似产品对比

### mprocs ⭐⭐⭐⭐⭐
- ✅ TUI 显示多个进程输出
- ✅ 可以切换查看不同进程
- ✅ 可以启动/停止/重启进程
- ❌ **没有统一的"控制面板"视图**
- ❌ **没有进度解析**

### procmux ⭐⭐⭐⭐
- ✅ YAML 配置驱动
- ✅ 支持信号服务器（HTTP API）
- ❌ **同样没有统一仪表盘**
- ❌ **没有进度感知**

### tmux ⭐⭐⭐
- ✅ 多窗口/分屏
- ✅ 可以发送命令
- ❌ **没有统一控制面板**
- ❌ **没有进度可视化**
- ❌ **操作不直观**

### GidTerm 的差异化：
1. ✅ **统一仪表盘** - 一眼看所有任务状态
2. ✅ **进度可视化** - 自动解析进度条/百分比
3. ✅ **语义控制** - 高级操作（保存模型、调参数）
4. ✅ **依赖调度** - 自动管理任务依赖关系
5. ✅ **智能建议** - 根据状态提供建议

---

## 💡 为什么这个想法有价值？

1. ✅ **现有工具都没有"统一仪表盘"概念**
2. ✅ **没人做进度自动解析 + 可视化**
3. ✅ **控制交互可以更直观**
4. ✅ **真实需求**（ML、批处理、微服务场景很常见）
5. ✅ **gid 提供了语义层** - project/task graph
6. ✅ **semantic parser 连接两者** - 理解输出，更新 graph

**这不仅是个 terminal multiplexer，而是：**
- Project orchestration tool
- Task dependency scheduler  
- Intelligent process supervisor
- Visual progress dashboard

---

---

## 🆚 和 Claude Code 的区别

### Claude Code 现在能做的：
- ✅ 可以并行运行多个命令
- ✅ 每个都有独立的 exec session
- ⚠️ 但它们是**独立的、无关联的**

### GidTerm 的核心区别：

#### 区别 1: 多项目支持 🎯

```
Claude Code:
所有 session 都在同一个 workspace
没有"项目"的概念

GidTerm:
┌─────────────────────────────────────┐
│ Project A: ML Pipeline              │
│   ├─ Task 1: Preprocess             │
│   ├─ Task 2: Train                  │
│   └─ Task 3: Evaluate               │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ Project B: Web App                  │
│   ├─ Task 1: Backend                │
│   ├─ Task 2: Frontend               │
│   └─ Task 3: Database               │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ Project C: Data Pipeline            │
│   ├─ Task 1: Extract                │
│   ├─ Task 2: Transform              │
│   └─ Task 3: Load                   │
└─────────────────────────────────────┘
```

**可以：**
- 切换不同 project 视图
- 每个 project 有自己的 task graph
- 跨 project 监控（"我有 3 个项目在跑，总体进度如何？"）

#### 区别 2: Task Graph vs 扁平进程 📊

```
Claude Code:
Session 1 ─┐
Session 2 ─┼─ 扁平列表，没有关系
Session 3 ─┘

GidTerm:
Download ──→ Preprocess ──┬──→ Train Model 1 ──┐
                          │                     ├──→ Evaluate
                          └──→ Train Model 2 ──┘

有依赖关系！有 DAG！
```

**意义：**
- ✅ 自动调度（A 完成 → 自动启动 B）
- ✅ 并行执行（B 和 C 可以同时跑）
- ✅ 可视化依赖（一眼看到瓶颈）

#### 区别 3: 语义理解 vs 纯文本 🧠

```
Claude Code:
看到的是原始输出：
> Epoch 45/100
> Loss: 0.234
（就是文本，没有理解）

GidTerm:
理解这是"训练任务"：
- Progress: 45% ← 自动提取
- Loss: 0.234 ← 结构化
- ETA: 23m ← 自动估算
- 提供专用操作：[Save Model] [Adjust LR]
```

#### 区别 4: 统一仪表盘 vs 逐个查看 📈

```
Claude Code:
要查看 3 个任务进度 → 需要切换 3 次
Session 1 → 看一下 → 切换
Session 2 → 看一下 → 切换
Session 3 → 看一下

GidTerm:
一个屏幕看所有：
╔════════════════════════════════════╗
║ [1] Preprocess    ████████  85% ⚙ ║
║ [2] Train Model 1 █████░░░  45% ⚙ ║
║ [3] Train Model 2 ████░░░░  38% ⚙ ║
╚════════════════════════════════════╝
```

#### 区别 5: 智能控制 vs 手动命令 🎮

```
Claude Code:
要保存模型 → 手动输入：
> model.save('checkpoint.pth')

GidTerm:
点击按钮 → 自动执行：
[Save Checkpoint] ← 点这个
  ↓
系统知道对于"ML Training"任务：
  → 应该发送 "model.save(...)"
```

### 对比总结表：

| 特性 | Claude Code | GidTerm |
|------|-------------|---------|
| **多进程支持** | ✅ | ✅ |
| **多项目管理** | ❌ | ✅ |
| **Task 依赖关系** | ❌ | ✅ DAG |
| **自动调度** | ❌ | ✅ |
| **进度可视化** | ❌ | ✅ |
| **语义理解** | ❌ | ✅ |
| **统一仪表盘** | ❌ | ✅ |
| **智能建议** | ❌ | ✅ |
| **高级控制** | ❌ | ✅ |

**核心区别：**
1. **Claude Code** = 多个独立的 terminal sessions（工具性）
2. **GidTerm** = 项目/任务编排系统（orchestration）

---

## 🧠 Semantic Level 详细展开

### 什么是 Semantic Level？

```
低层控制：
"我知道这是一个进程"
"我能发送信号给它"

Semantic 控制：
"我知道这是什么类型的任务"
"我知道它现在处于什么状态"
"我知道现在可以做什么操作"
"我知道什么时候该建议用户做什么"
"我能理解输出的含义"
"我能提供高级抽象的控制"
```

### 对比：语义层面 vs 低层控制

#### 低层工具看到的：
```
┌─────────────────────────┐
│ python train.py         │
│ > Epoch 45/100          │ ← 只是文本
│ > Loss: 0.234           │
└─────────────────────────┘

控制方式：
- 发送 Ctrl+C (杀进程)
- 发送原始文本命令
```

#### Semantic 层看到的：
```
┌─────────────────────────┐
│ 🧠 ML Training Task     │ ← 理解这是"训练任务"
│ Model: ResNet50         │ ← 提取语义信息
│ Progress: 45%           │
│ Current Loss: 0.234     │
│ Est. Time: 23m          │
└─────────────────────────┘

控制方式：
- [Save Checkpoint]  ← 语义级操作
- [Adjust LR]
- [Early Stop]
- [Resume Training]
```

### 层次对比表：

| 层次 | 低层工具 | Semantic 工具 |
|------|----------|---------------|
| **控制层** | 原始命令 (Ctrl+C, 文本) | 语义操作 (保存模型、调整参数) |
| **理解层** | 纯文本输出 | 结构化数据提取 |
| **交互层** | 手动输入命令 | 智能建议 + 一键操作 |
| **可视化** | 文本流 | 结构化指标 + 图表 |

---

## 🔍 Semantic Level 1: 任务类型识别

### 问题：
```
现有工具看到的：
> python train.py
> npm run build  
> docker-compose up

都只是"一个进程"
```

### Semantic 做的：
```yaml
tasks:
  train:
    type: ml_training  # ← 标记类型
    
  build:
    type: build_task
    
  database:
    type: service
```

### 为什么重要：

不同类型的任务，需要**不同的控制方式**：

```rust
match task.type {
    TaskType::MLTraining => {
        // ML 训练可以：
        actions: [
            "Save Checkpoint",
            "Adjust Learning Rate", 
            "Early Stop",
            "Plot Loss Curve"
        ]
        parser: MLTrainingParser
    }
    
    TaskType::BuildTask => {
        // 构建任务可以：
        actions: [
            "Skip Tests",
            "Clean Build",
            "Incremental Build"
        ]
        parser: BuildOutputParser
    }
    
    TaskType::Service => {
        // 服务可以：
        actions: [
            "Hot Reload",
            "Health Check",
            "View Logs",
            "Restart"
        ]
        parser: ServerLogParser
    }
    
    TaskType::DataProcessing => {
        // 数据处理可以：
        actions: [
            "Pause/Resume",
            "Skip Current Batch",
            "View Sample Output"
        ]
        parser: DataPipelineParser
    }
}
```

### 具体例子：

```
场景：训练模型时 loss 爆了

低层工具：
你只能：
1. Ctrl+C 杀掉进程
2. 手动改代码
3. 重新运行

Semantic 工具：
系统知道这是"ML Training"：
1. 检测到 loss = NaN
2. 自动暂停训练
3. 提示："检测到异常，建议降低学习率"
4. 提供按钮：[降低 LR] [回到上个 checkpoint]
```

---

## 🗣️ Semantic Level 2: 智能命令翻译

### 问题：
```
用户想要：保存模型

低层方式：
必须知道具体命令：
> model.save_checkpoint('checkpoint_epoch45.pth')
> torch.save(model.state_dict(), 'model.pth')
> joblib.dump(model, 'model.pkl')
（不同框架，命令不同）
```

### Semantic 做的：

**配置文件定义"语义命令"：**
```yaml
task: train_model
  semantic_commands:
    # 用户友好的名字 → 实际命令
    save: "model.save('checkpoint.pth')"
    
    adjust_lr: |
      optimizer.param_groups[0]['lr'] = {value}
      print(f"LR adjusted to {value}")
    
    plot_metrics: |
      import matplotlib.pyplot as plt
      plt.plot(losses)
      plt.savefig('loss.png')
      
    early_stop: "trainer.should_stop = True"
```

### UI 交互流程：

**步骤 1: 显示可用操作**
```
╔════════════════════════════════╗
║  Task Actions:                 ║
║  [💾 Save Checkpoint]          ║  ← 点这个
║  [📉 Plot Metrics]              ║
║  [⚙️  Adjust LR]                ║
╚════════════════════════════════╝
```

**步骤 2: 参数输入（如需要）**
```
点击 [Adjust LR] 
  ↓
弹出输入框：
┌────────────────────┐
│ New Learning Rate: │
│ [0.0001___]        │
│ [OK] [Cancel]      │
└────────────────────┘
```

**步骤 3: 自动执行**
```
输入 0.0001 → OK
  ↓
自动执行：
> optimizer.param_groups[0]['lr'] = 0.0001
> print(f"LR adjusted to 0.0001")

Terminal 显示：
LR adjusted to 0.0001
```

### 实现代码：

```rust
struct SemanticCommand {
    label: String,         // UI 上显示的
    template: String,      // 实际命令模板
    params: Vec<Param>,    // 需要的参数
}

struct Param {
    name: String,
    param_type: ParamType,  // String/Float/Int/Bool
    default: Option<Value>,
}

// 用户点击 "Adjust LR"
fn execute_semantic_command(
    cmd: &SemanticCommand, 
    params: HashMap<String, Value>
) {
    // 替换模板中的参数
    let actual_cmd = cmd.template
        .replace("{value}", &params["value"].to_string());
    
    // 发送到 terminal
    terminal.send_command(&actual_cmd);
}
```

### 更高级：LLM 驱动的命令翻译

```rust
// 用户自然语言输入
user_input: "把学习率降低一半"

// LLM 翻译
let command = llm.translate(
    user_input, 
    context: {
        task_type: "ml_training",
        current_lr: 0.001,
        framework: "pytorch"
    }
);

// 生成命令
command = "optimizer.param_groups[0]['lr'] = 0.0005"

// 执行前确认
UI: "将执行: optimizer.param_groups[0]['lr'] = 0.0005"
    [Confirm] [Edit] [Cancel]
```

---

## 🎯 Semantic Level 3: 上下文理解

### 问题：
```
低层工具：
不知道任务处于什么状态
用户要自己判断"现在能做什么"
```

### Semantic 做的：

```rust
struct TaskContext {
    status: TaskStatus,      // Running/Paused/Done/Failed
    progress: f32,           // 0.0 - 1.0
    current_phase: Phase,    // Init/Loading/Training/Validating
    metrics: HashMap<String, f32>,
    errors: Vec<Error>,
}

impl TaskContext {
    // 根据上下文，决定可以做什么
    fn get_available_actions(&self) -> Vec<Action> {
        match (self.status, self.current_phase) {
            // 正在训练中
            (Running, Phase::Training) => vec![
                Action::Pause,
                Action::SaveCheckpoint,
                Action::AdjustHyperparams,
                Action::ViewMetrics,
            ],
            
            // 已暂停
            (Paused, _) => vec![
                Action::Resume,
                Action::ModifyConfig,  // 只有暂停时才能改配置
                Action::Kill,
            ],
            
            // 训练完成
            (Done, Phase::Training) => vec![
                Action::ViewResults,
                Action::ExportModel,
                Action::StartEvaluation,  // 可以触发下一步
            ],
            
            // 出错了
            (Failed, _) => vec![
                Action::ViewErrorLog,
                Action::DiagnoseIssue,
                Action::RetryWithFix,
            ],
        }
    }
}
```

### UI 根据上下文动态变化：

**状态 1: 正在训练**
```
╔════════════════════════════════╗
║  ResNet Training - Running ⚙   ║
║  Progress: 45% | Loss: 0.234   ║
║                                ║
║  [⏸️ Pause]  [💾 Save]  [📊 Plot] ║
╚════════════════════════════════╝
```

**状态 2: 已暂停**
```
╔════════════════════════════════╗
║  ResNet Training - Paused ⏸    ║
║  Progress: 45% (paused)        ║
║                                ║
║  [▶️ Resume]  [⚙️ Modify Config] ║
║  [❌ Kill]                      ║
╚════════════════════════════════╝
```

**状态 3: 完成**
```
╔════════════════════════════════╗
║  ResNet Training - Done ✓      ║
║  Final Loss: 0.123 | Acc: 94%  ║
║                                ║
║  [📁 Export]  [🔬 Evaluate]     ║
║  [🔄 Retrain]                  ║
╚════════════════════════════════╝
```

**状态 4: 出错**
```
╔════════════════════════════════╗
║  ResNet Training - Failed ❌    ║
║  Error: CUDA out of memory     ║
║                                ║
║  💡 Suggestion:                ║
║  • Reduce batch size           ║
║  • Use gradient accumulation   ║
║                                ║
║  [📋 View Log]  [🔧 Fix & Retry] ║
╚════════════════════════════════╝
```

---

## 📊 Semantic Level 4: 结构化输出解析

### 问题：
```
低层工具看到的：
Epoch 45/100 | Loss: 0.234 | Acc: 0.876 | LR: 0.001
Batch 1234/1400
Time elapsed: 2h34m

（只是文本）
```

### Semantic 做的：

**解析器实现：**
```rust
struct MLOutputParser {
    patterns: Vec<Regex>,
}

impl OutputParser for MLOutputParser {
    fn parse(&self, output: &str) -> ParsedMetrics {
        // 正则匹配
        let epoch_re = Regex::new(r"Epoch (\d+)/(\d+)").unwrap();
        let loss_re = Regex::new(r"Loss: ([\d.]+)").unwrap();
        let acc_re = Regex::new(r"Acc: ([\d.]+)").unwrap();
        
        // 提取结构化数据
        ParsedMetrics {
            progress: epoch as f32 / total_epochs as f32,
            metrics: hashmap!{
                "epoch" => epoch,
                "total_epochs" => total_epochs,
                "loss" => loss,
                "accuracy" => accuracy,
                "learning_rate" => lr,
            },
            timestamp: now(),
        }
    }
}
```

### 提取后可以做什么：

#### 1. 实时可视化
```
Loss 历史：
0.5 ┤╮
0.4 ┤ ╰╮
0.3 ┤   ╰╮
0.2 ┤     ╰─── ← 当前
    └────────────
    0  25 50 75 100
    
Accuracy 历史：
100%┤          ╭─ ← 当前
 75%┤      ╭──╯
 50%┤  ╭──╯
    └────────────
```

#### 2. 智能警报
```rust
// 规则引擎
if metrics.loss > 1.0 && progress > 0.2 {
    alert("Loss 还很高，可能有问题");
}

if metrics.loss.is_nan() {
    alert("Loss 变成 NaN，立即停止！");
    auto_pause();
}

if !metrics.loss_decreased_in_last(20, epochs) {
    suggest("Loss 没下降，考虑调整学习率");
}
```

#### 3. 趋势分析
```rust
// 计算 loss 下降速度
let loss_velocity = calculate_derivative(loss_history);

if loss_velocity.abs() < 0.001 {
    suggest("Loss 下降变慢了，可能快收敛了");
}

// 预估完成时间
let remaining_epochs = total_epochs - current_epoch;
let avg_time_per_epoch = total_time / current_epoch;
let eta = remaining_epochs * avg_time_per_epoch;

display(f"ETA: {eta.human_readable()}");
```

#### 4. 自动决策
```rust
// 基于规则的自动操作
if metrics.accuracy > 0.95 && metrics.loss < 0.1 {
    suggest_action("模型效果已经很好，可以提前停止");
    
    if user_config.auto_stop_enabled {
        auto_stop();
        save_checkpoint("best_model.pth");
        notify("训练已自动停止并保存最佳模型");
    }
}
```

### 完整可视化界面：

```
╔════════════════════════════════════════╗
║  ResNet Training Metrics               ║
╠════════════════════════════════════════╣
║                                        ║
║  Loss:                                 ║
║   0.5 ┤                                ║
║   0.4 ┤      ╲                         ║
║   0.3 ┤       ╲___                     ║
║   0.2 ┤           ╲_____ ← current     ║
║   0.1 ┤                                ║
║       └─────────────────────           ║
║        0    50   100  150  200 epochs  ║
║                                        ║
║  Current Metrics:                      ║
║  • Epoch: 178/200 (89%)                ║
║  • Loss: 0.234 ↓                       ║
║  • Accuracy: 87.6% ↑                   ║
║  • Learning Rate: 0.001                ║
║  • ETA: 23 minutes                     ║
║                                        ║
║  Trend: ✅ Converging normally         ║
╚════════════════════════════════════════╝
```

---

## 🌐 Semantic Level 5: 跨任务理解

### 问题：
```
低层工具：
3 个独立的进程
互不相关
```

### Semantic 做的：

**理解任务之间的关系：**
```rust
struct CrossTaskContext {
    tasks: Vec<Task>,
    relations: Vec<Relation>,
}

// 示例：比较不同模型
impl CrossTaskContext {
    fn compare_models(&self) -> Comparison {
        let model_a = self.tasks[0].metrics;  // ResNet
        let model_b = self.tasks[1].metrics;  // EfficientNet
        let model_c = self.tasks[2].metrics;  // ViT
        
        Comparison {
            best_loss: model_b,     // EfficientNet loss 最低
            best_accuracy: model_a, // ResNet 准确率最高
            fastest: model_c,       // ViT 训练最快
            
            recommendation: "ResNet 准确率最高但速度慢，
                           EfficientNet 提供最佳平衡"
        }
    }
    
    fn detect_anomalies(&self) -> Vec<Anomaly> {
        // 检测异常
        if model_a.loss > model_b.loss * 2.0 {
            return vec![Anomaly {
                task: "ResNet",
                issue: "Loss 明显高于其他模型",
                suggestion: "检查数据或超参数配置"
            }];
        }
    }
}
```

### UI 显示：

**模型对比视图：**
```
╔════════════════════════════════════════╗
║  Model Comparison                      ║
╠════════════════════════════════════════╣
║                                        ║
║  Model       Loss    Acc    Speed      ║
║  ────────────────────────────────────  ║
║  ResNet      0.234   94.2%  slow       ║
║  EfficientNet 0.189  92.8%  medium  ⭐ ║
║  ViT         0.267   91.5%  fast       ║
║                                        ║
║  💡 Recommendation:                    ║
║  EfficientNet 提供最佳平衡             ║
║                                        ║
║  [Export Best] [Continue All] [Stop]   ║
╚════════════════════════════════════════╝
```

**依赖关系智能分析：**
```
╔════════════════════════════════════════╗
║  Pipeline Analysis                     ║
╠════════════════════════════════════════╣
║                                        ║
║  Bottleneck: Data Preprocessing        ║
║  • Taking 45 min (expected: 20 min)    ║
║  • Blocking 2 downstream tasks         ║
║                                        ║
║  💡 Suggestions:                       ║
║  • Increase preprocessing workers      ║
║  • Cache intermediate results          ║
║  • Consider parallel preprocessing     ║
╚════════════════════════════════════════╝
```

---

## 🤖 Semantic Level 6: LLM 增强（终极形态）

### 概念：用 LLM 理解任务输出和状态

**传统 Parser：**
```rust
// 需要为每种输出格式写正则
let loss_re = Regex::new(r"Loss: ([\d.]+)").unwrap();
```

**LLM Parser：**
```rust
struct LLMParser {
    llm_client: LLMClient,
}

impl OutputParser for LLMParser {
    fn parse(&self, output: &str) -> ParsedMetrics {
        let prompt = format!(
            "Parse the following training output and extract metrics as JSON:
            
            Output: {}
            
            Return JSON with fields: epoch, loss, accuracy, etc.",
            output
        );
        
        let response = self.llm_client.complete(prompt);
        serde_json::from_str(&response).unwrap()
    }
}
```

### LLM 驱动的智能建议：

```rust
struct LLMAdvisor {
    llm: LLMClient,
    context: TaskContext,
}

impl LLMAdvisor {
    fn analyze_and_suggest(&self) -> Suggestion {
        let prompt = format!(
            "You are a ML training expert. Analyze this situation:
            
            Task: {}
            Current Metrics: {:?}
            Recent History: {:?}
            
            Provide actionable suggestions.",
            self.context.task_name,
            self.context.current_metrics,
            self.context.metrics_history
        );
        
        let suggestion = self.llm.complete(prompt);
        
        Suggestion {
            text: suggestion,
            actions: self.extract_suggested_actions(&suggestion),
        }
    }
}
```

### UI 交互：

```
╔════════════════════════════════════════╗
║  🤖 AI Assistant                       ║
╠════════════════════════════════════════╣
║                                        ║
║  I noticed your training loss hasn't   ║
║  improved in the last 15 epochs.       ║
║                                        ║
║  Possible causes:                      ║
║  1. Learning rate too low (current: 0.001)
║  2. Model may be stuck in local minimum║
║  3. Dataset may need shuffling         ║
║                                        ║
║  Suggested actions:                    ║
║  • [Try LR=0.01] (10x increase)        ║
║  • [Add learning rate scheduler]       ║
║  • [Restart with momentum optimizer]   ║
║                                        ║
║  Would you like me to apply any?       ║
╚════════════════════════════════════════╝
```

---

## 🎯 Semantic 的本质

```
低层控制：
"我知道这是一个进程"
"我能发送信号给它"

Semantic 控制：
"我知道这是什么类型的任务"
"我知道它现在处于什么状态"
"我知道现在可以做什么操作"
"我知道什么时候该建议用户做什么"
"我能理解输出的含义"
"我能提供高级抽象的控制"
```

### 类比：

```
低层 = 机器语言
    你必须知道每个 bit 的含义

Semantic = 高级编程语言
    你用人类可理解的抽象概念

---

低层 = 直接操作硬件
    设置寄存器、管理内存地址

Semantic = 操作系统提供的抽象
    文件、进程、网络 socket
```

### 核心价值：

1. **降低认知负担** - 不需要记住复杂命令
2. **减少错误** - 系统知道什么时候能做什么
3. **提高效率** - 一键操作 vs 手动输入
4. **智能辅助** - 主动发现问题和建议
5. **可扩展性** - 新任务类型只需添加 parser

---

## 📋 实现优先级

### Phase 1: 基础架构（必需）
1. ✅ PTY 管理和多 terminal 控制
2. ✅ Task graph 和依赖调度
3. ✅ 基本的进度解析（正则匹配）
4. ✅ 统一仪表盘 UI

### Phase 2: 语义层核心（必需）
1. ✅ 任务类型识别系统
2. ✅ 语义命令定义和执行
3. ✅ 上下文感知的动作菜单
4. ✅ 结构化指标提取

### Phase 3: 智能增强（重要）
1. ✅ 智能建议系统（基于规则）
2. ✅ 跨任务分析和对比
3. ✅ 趋势分析和 ETA 预测
4. ✅ 自动决策（可选启用）

### Phase 4: 高级功能（可选）
1. ⚠️ LLM 驱动的解析和建议
2. ⚠️ 多项目管理
3. ⚠️ 远程控制 API
4. ⚠️ 插件系统

---

## 🔄 **和 IdeaSpark 的关系** ⭐ NEW

### **定位：GidTerm 是 IdeaSpark 的执行引擎**

```
IdeaSpark (完整产品)
├── Idea 管理 (现有)
├── AI 分类 (现有)
├── Task Graph (现有)
└── Terminal 执行层 ← GidTerm
    ├── PTY 管理
    ├── 实时监控
    └── 语义解析
```

**两种使用场景：**

#### **场景 1: 配合 IdeaSpark**
```
用户: IdeaSpark 用户
流程: IdeaSpark 生成 graph.yml → GidTerm 执行
```

#### **场景 2: 独立使用**
```
用户: 任何开发者（没用 IdeaSpark）
流程: 手写 config → GidTerm 执行
```

**开发策略：**
- ✅ **现在**：独立开发 GidTerm，保持接口清晰
- ✅ **未来**：作为模块集成进 IdeaSpark
- ✅ **设计**：核心库 + CLI + 集成层（WASM/FFI）

---

## 📝 **配置文件格式** ⭐ DECIDED

### **支持多种格式（自动识别）**

#### **格式 1: 超简化（快速开始）**
```yaml
# 纯命令列表
tasks:
  dev: npm run dev
  test: npm test
  build: npm run build
```

#### **格式 2: 标准格式（推荐）**
```yaml
# 手写友好，支持依赖和类型
project: my-project

tasks:
  build:
    command: npm run build
    type: build
    
  test:
    command: npm test
    depends_on: [build]
    type: test_suite
    
  deploy:
    command: ./deploy.sh
    depends_on: [test]
```

#### **格式 3: IdeaSpark 完整格式（兼容）**
```yaml
# 完整兼容 IdeaSpark 的 .gid/graph.yml
nodes:
  build:
    type: Task
    description: Build the project
    command: npm run build
    parser: build_parser
    status: pending
    created_at: 2026-01-30
    semantic_commands:
      clean: "rm -rf dist/"
```

**文件名优先级：**
1. `project.gid.yml` - 手写的标准配置
2. `.gid/graph.yml` - IdeaSpark 格式
3. `gidterm.yml` - 备选

---

## 🎯 **技术决策** ⭐ DECIDED

### **核心技术栈：**
- **语言**: Rust
  - 性能优秀
  - 可编译成 WASM（供 IdeaSpark 集成）
  - 类型安全
  
- **配置格式**: YAML
  - 和 IdeaSpark 一致
  - 用户友好
  - 生态成熟（serde_yaml）
  
- **TUI 框架**: ratatui + crossterm
  - 现代化 Rust TUI 框架
  - 活跃维护
  
- **PTY 库**: portable-pty
  - 跨平台抽象
  - 可靠成熟
  
- **Graph 库**: 自定义实现
  - 符合特定需求
  - 与 gid 工具链兼容

### **Parser 策略：分层**

```
Layer 1: Regex（快速，MVP）
  ├─ 通用进度条解析
  ├─ 百分比提取
  └─ 基础模式匹配

Layer 2: LLM（智能，未来）
  └─ 复杂/模糊输出理解
```

---

## 🏗️ **项目架构** ⭐ UPDATED

```
gidterm/
├── .gid/                   # Project graph (gid MCP)
│   └── graph.yml           # Components + Tasks
├── .mcp.json               # MCP server config
├── src/
│   ├── core/               # 核心引擎（可被集成）
│   │   ├── graph.rs        # Graph 解析
│   │   ├── pty.rs          # PTY 管理
│   │   ├── scheduler.rs    # 任务调度
│   │   └── lib.rs
│   ├── semantic/           # 语义层
│   │   ├── registry.rs     # Parser 注册
│   │   ├── commands.rs     # 语义命令
│   │   └── parsers/
│   │       ├── regex.rs
│   │       ├── ml_training.rs
│   │       └── build.rs
│   ├── ui/                 # CLI + TUI
│   │   ├── cli.rs          # 命令行接口
│   │   ├── tui.rs          # TUI 框架
│   │   └── views/
│   │       ├── dashboard.rs
│   │       ├── graph.rs
│   │       └── terminal.rs
│   ├── bindings/           # 集成层（未来）
│   │   ├── wasm/           # Web 集成
│   │   └── ffi/            # 其他语言
│   └── main.rs
├── examples/               # 示例配置
├── docs/
└── tests/
```

---

## 🌐 **Multi-Project Developer Experience (DX)** ⭐ RFC

*基于 Theo (t3.gg) 的痛点，设计 gidterm 的多项目管理能力*

### **Motivation: The Multi-Project Problem**

来自 Theo 的推文指出的核心痛点：

> "The biggest thing that sucks about working with Coding Agents on multiple projects is keeping track of what's happening. I get the multiple terminal tabs all look the same, multiple browser tabs with different localhosts..."

**核心问题：**
1. 🔍 **可见性差** - 多个 terminal tabs，哪个 agent 完成了？找不到
2. 🔌 **Port 冲突** - localhost:3000 被谁占了？
3. 🌐 **Browser 混乱** - 哪个 chrome 窗口是哪个项目的？
4. 🧠 **心智负担** - 单项目能记住，多项目完全乱
5. ⏱️ **Context 切换** - 开销大于实际 coding 时间

Theo 说他 "almost started to build an OS" 来解决这个问题 - 我们不需要 OS 级别，但 gidterm 作为 terminal controller 已经有了基础，可以成为解决方案。

### **Current State**

gidterm 已经有的能力：
- ✅ Multi-project workspace mode (`--workspace`)
- ✅ 项目隔离（每个项目独立 graph）
- ✅ Task DAG scheduling
- ✅ Parallel execution
- ✅ Real-time TUI dashboard

缺失的：
- ❌ 全局项目状态概览
- ❌ Port 管理/追踪
- ❌ Agent 状态集成
- ❌ 通知聚合
- ❌ 浏览器集成

### **Proposed Features**

#### 1. 🎛️ Unified Dashboard

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
- 每行显示：项目名、分配的 port、agent 状态、task pipeline 概览
- 底部显示最近事件，highlight 需要注意的
- 颜色编码：🟢运行中 🔵需输入 🟡警告 🔴错误 ⏸️暂停

#### 2. 🔌 Port Management

自动管理开发服务器端口，避免冲突：

```yaml
# .gid/graph.yml 中的 port 配置
metadata:
  project: "backend"
  port: auto          # gidterm 自动分配
  # 或者
  port: 3000          # 首选 port
  port_fallback: true # 冲突时自动 +1
```

**功能：**
- 自动扫描 `3000-3999` 范围找可用 port
- 维护全局 port registry（`~/.gidterm/ports.json`）
- 启动时注入 `$PORT` 环境变量
- Port 冲突检测和自动解决
- `gidterm ports` 命令查看当前分配

```bash
$ gidterm ports
PORT    PROJECT         PROCESS         STATUS
3000    backend         npm run dev     🟢 active
3001    frontend        vite            🟢 active  
3002    api-gateway     -               ⏸️ reserved
```

#### 3. 🤖 Agent Integration

与 coding agent 深度集成：

**支持的 agents：**
- Claude Code (`claude`)
- Codex CLI (`codex`)
- OpenCode (`opencode`)
- Pi Coding Agent

**集成方式：**
```yaml
# .gid/graph.yml
tasks:
  implement-feature:
    agent: claude          # 指定使用哪个 agent
    prompt: "Implement user authentication"
    status: pending
```

**状态追踪：**
- 检测 agent 进程是否运行
- 解析 agent 输出判断状态（running/waiting/completed/error）
- Agent 完成时触发通知

#### 4. 🔔 Notification Aggregation

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
  on_waiting: true      # agent 等待输入时
  sound: true
  channels:
    - system            # OS notification
    - telegram          # 可选
```

#### 5. ⚡ Quick Switch

快速在项目间切换：

```bash
# CLI 方式
gidterm switch backend      # 聚焦到 backend 项目
gidterm focus frontend      # 同上

# TUI 中
# 按 1/2/3 数字键快速切换
# 或 / 搜索项目名
```

**切换时的动作：**
- 将该项目的 terminal 带到前台
- 打开相关的 browser tabs（如果有集成）
- 更新 TUI 焦点

#### 6. 🌐 Browser Integration (Phase 2)

通过 Chrome Extension 实现 tab 分组：

**功能：**
- 按项目自动分组 tabs
- 识别 `localhost:PORT` 并关联到项目
- 一键打开项目的所有相关 URLs
- 关闭项目时可选关闭相关 tabs

**实现路径：**
1. Chrome Extension 监听 tab 创建
2. Extension 与 gidterm 通过 WebSocket 通信
3. gidterm 发送 port-project 映射
4. Extension 自动给 tabs 打标签/分组

```
Chrome Tab Groups:
├── 📁 backend (localhost:3000)
│   ├── App - localhost:3000
│   └── API Docs - localhost:3000/docs
├── 📁 frontend (localhost:3001)
│   └── Vite - localhost:3001
└── Other tabs...
```

### **Implementation Plan**

#### Phase 1: Core DX ✅ DONE (2026-02-03)
- [x] Unified dashboard redesign — `src/ui/views/project_overview.rs`
- [x] Port management system — `src/ports.rs` (PortRegistry, PortManager, ~/.gidterm/ports.json)
- [x] Basic notifications (macOS) — `src/notifications.rs` (NotificationManager, osascript)
- [x] Quick switch (keyboard shortcuts) — 1-9 keys, `/` search, ←→ navigation

#### Phase 2: Agent Integration (2 weeks)
- [ ] Agent process detection
- [ ] Agent status parsing
- [ ] Agent task definition in graph

#### Phase 3: Browser Integration (2-3 weeks)
- [ ] Chrome Extension scaffold
- [ ] WebSocket bridge
- [ ] Tab grouping logic
- [ ] URL-to-project mapping

#### Phase 4: Polish (1 week)
- [ ] Configuration system
- [ ] Documentation
- [ ] Demo video

### **Open Questions**

1. **Port persistence** - 每次启动用同样的 port 还是 fresh 分配？
   - 建议：持久化，但检测冲突时重新分配

2. **Agent detection** - 如何判断 agent 状态？
   - 解析 stdout 关键词？检测进程？Agent API？
   - 建议：先做进程检测 + stdout 关键词，后续可以加 API

3. **Cross-platform** - 是否支持 Linux/Windows？
   - macOS 优先，Linux 次之，Windows 低优先级

4. **与其他工具的关系** - tmux/Warp/iTerm？
   - gidterm 是独立 TUI，不依赖也不替代这些工具
   - 可以在 tmux 里运行 gidterm

### **Alternatives Considered**

1. **VS Code Extension** - 更深的 IDE 集成
   - 缺点：绑定 VS Code，不够通用

2. **Electron App** - 图形界面
   - 缺点：重，开发成本高

3. **tmux wrapper** - 包装 tmux
   - 缺点：tmux 学习曲线，配置复杂

选择 TUI 的原因：轻量、跨终端、符合开发者习惯

### **Success Metrics**

- 项目切换时间 < 2 秒
- Port 冲突率 → 0
- "找 agent" 的时间 → 0（直接看 dashboard）
- 用户不再需要肉眼扫描多个 terminal tabs

### **References**

- [Theo's tweet thread](https://twitter.com/t3dotgg/...)
- [mprocs](https://github.com/pvolok/mprocs) - 多进程 TUI 参考
- [Chrome Tab Groups API](https://developer.chrome.com/docs/extensions/reference/tabGroups/)

---

## 📋 **开发路线图** ⭐ UPDATED 2026-01-31

### **Phase 1: 核心引擎 ✅ DONE**
**目标：独立可用的 GidTerm CLI**

- [x] 项目初始化（Cargo + Git）
- [x] Graph 解析器（.gid/graph.yml）— `src/core/graph.rs`
- [x] PTY 管理器（创建/控制/I/O）— `src/core/pty.rs`
- [x] 任务调度器（DAG + 依赖）— `src/core/scheduler.rs`
- [x] 基础 TUI（任务列表 + 状态）— `src/ui/live.rs`
- [x] P0 Bug fixes: `sh -c` wrapping, process lifecycle, async blocking, exit codes
- [x] GraphTaskStatus enum (replaced raw strings)
- [x] Session persistence — `src/session.rs`
- [x] Multi-project workspace — `src/workspace.rs`

### **Phase 2: 语义层 ✅ DONE**
**目标：智能理解任务输出**

- [x] Parser 注册系统 — `src/semantic/registry.rs`
- [x] Regex-based parsers — `src/semantic/parsers/regex.rs`
- [x] ML training parser — `src/semantic/parsers/ml_training.rs`
- [x] Build task parser — `src/semantic/parsers/build.rs`
- [x] 语义命令模板 — `src/semantic/commands.rs`
- [x] Wired parsers into App event loop
- [x] TUI progress bars + inline metrics

### **Phase 3: 高级 UI ✅ DONE**
**目标：完整的用户体验**

- [x] Dashboard 视图（统一仪表盘）— `src/ui/live.rs` + `src/ui/dashboard.rs`
- [x] Graph 视图（可视化 DAG）— `src/ui/views/graph.rs`
- [x] Terminal 视图（全屏终端 + semantic controls）— `src/ui/views/terminal.rs`
- [x] 实时进度追踪 (progress bars, metrics)
- [x] View switching: Tab cycle, 1/2/3/4 keys, Enter for terminal
- [x] ETA 计算 — `src/semantic/history.rs`
- [x] MetricHistory + trend tracking — `src/semantic/history.rs`
- [x] SmartAdvisor (rule-based advisories) — `src/semantic/advisor.rs`
- [x] Sparkline charts in terminal view — `src/ui/views/terminal.rs`
- [x] Cross-task comparison view — `src/ui/views/comparison.rs`
- [x] Clap CLI with subcommands (run, status, init, history, start) — `src/main.rs`

### **Phase 4: AI Integration ✅ DONE**
**目标：支持三种控制模式**

- [x] ControlAPI trait — `src/ai/control.rs`
- [x] ControlMode enum (Manual/MCP/Agent)
- [x] JSON event streaming (GidEvent + EventStream) — `src/ai/events.rs`
- [x] ControlCommand/ControlResponse serialization
- [x] StateSnapshot for AI consumers

### **Phase 5: 集成准备（未来）**
**目标：可被 IdeaSpark 调用**

- [ ] WASM 编译
- [ ] MCP server mode (gidterm as MCP tool provider)
- [ ] Clawdbot automation driver
- [ ] 文档化接口

---

## 🚀 **快速开始设计** ⭐ NEW

### **三种入口，渐进式复杂度：**

#### **Level 1: 超简单（5 秒）**
```bash
# 不需要任何配置
gidterm run "npm run dev" "npm test"
```

#### **Level 2: 标准使用（推荐）**
```bash
# 1. 初始化
gidterm init

# 2. 编辑配置
vim project.gid.yml

# 3. 运行
gidterm start
```

#### **Level 3: IdeaSpark 集成**
```bash
# 在 IdeaSpark 项目目录
gidterm start

# 或指定路径
gidterm start --graph /path/to/.gid/graph.yml
```

---

## 🔗 **API 设计（供集成）** ⭐ NEW

```rust
// GidTerm 核心 API（未来供 IdeaSpark 调用）
pub struct GidTermEngine {
    graph: TaskGraph,
    terminals: TerminalManager,
    parsers: ParserRegistry,
}

impl GidTermEngine {
    // 从 graph.yml 初始化
    pub fn from_graph(path: &Path) -> Result<Self>;
    
    // 启动任务
    pub fn start_task(&mut self, task_id: &str) -> Result<TaskHandle>;
    
    // 获取实时状态
    pub fn get_status(&self, task_id: &str) -> TaskStatus;
    
    // 发送命令
    pub fn send_command(&mut self, task_id: &str, cmd: &str);
    
    // 订阅事件
    pub fn on_progress<F>(&mut self, callback: F);
}
```

---

## 📊 **Graph 维护策略** ⭐ NEW

**使用 gid MCP tool 管理两个 graph：**

### **1. Project Graph（架构）**
```yaml
# .gid/graph.yml - nodes 部分
nodes:
  GraphParser:
    type: Component
    layer: core
    status: in-progress
    path: src/core/graph.rs
```

### **2. Task Graph（开发任务）**
```yaml
# .gid/graph.yml - tasks 部分
tasks:
  implement_graph_parser:
    type: Development
    status: in-progress
    component: GraphParser
    depends_on: [setup_rust_project]
```

**更新规则：**
- 开发时：通过 gid MCP 更新状态
- 完成组件：node.status → active
- 完成任务：task.status → done
- 保持同步：定期 commit graph.yml

---

## 📋 待研究问题

1. ~~和 Claude Code 的区别？~~ ✅ 已明确
2. ~~Semantic level 的详细定义和实现~~ ✅ 已展开
3. ~~配置文件格式设计~~ ✅ 已决定（YAML 多格式）
4. ~~Parser 库的选择和实现~~ ✅ 已决定（分层策略）
5. ~~MVP 最小功能集确定~~ ✅ 已规划
6. ~~技术栈最终选择~~ ✅ 已决定（Rust）
7. ~~和 IdeaSpark 的关系~~ ✅ 已明确（执行引擎）
8. ~~State persistence 策略~~ ✅ 已决定（JSON in .gidterm/sessions/）
9. ~~Multi-project UI 布局~~ ✅ 已实现（workspace mode + project grouping）

**剩余问题：**
- Remote control API 设计
- LLM-powered parser (Phase 4)
- Plugin system for custom parsers

---

## 📊 **实现状态** ⭐ v0.4.0

| Layer | Coverage | Status |
|-------|----------|--------|
| Core Engine | 95% | ✅ GraphParser, PTYManager, Scheduler, Executor, GraphTaskStatus enum |
| Semantic Layer | 95% | ✅ ParserRegistry, RegexParser, MLTrainingParser, BuildParser, SemanticCommands, MetricHistory, SmartAdvisor |
| Terminal UI | 98% | ✅ LiveDashboard, TerminalView, GraphView, ComparisonView, **ProjectOverview**, Sparklines, view switching |
| AI Integration | 90% | ✅ ControlAPI trait, ControlMode (Manual/MCP/Agent), EventStream, ControlCommand/Response |
| CLI | 95% | ✅ Clap subcommands: run, status, init, history, start, **ports** |
| Multi-project DX | 95% | ✅ **Phase 1 Done**: UnifiedDashboard, PortManager, NotificationSystem, QuickSwitch |
| Session | 90% | ✅ SessionManager, task history, output tracking |
| Tests | 85% | ✅ 59 tests (43 unit + 16 integration), 0 failures |

**GID Graph Health Score: 95/100** (graph-indexed-development-mcp)
**Graph Nodes: 33** (7 Features, 24 Components, 2 Tests)

---

*记录时间：2026-01-30*
*最后更新：2026-02-03*
*开发工具：Claude Code (Opus 4.5) + graph-indexed-development-mcp*
