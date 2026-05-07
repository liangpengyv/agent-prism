# AgentPrism 设计文档

**日期**：2026-05-07  
**版本**：V0.1 → V1.0 全程规划  
**技术栈**：Tauri 2 + Vue 3 + Rust  
**平台优先级**：macOS 优先

---

## 1. 产品定位

AgentPrism（算力棱镜）是一款常驻桌面的本地 AI Agent Token 消耗监控工具。它不上传任何数据，所有分析逻辑 100% 在本地闭环。核心定位是"精密测量仪器"，而非生产力工具。

MVP 先做 Codex，V1.0 平行接入 Claude Code。

---

## 2. 整体架构

```
┌─────────────────────────────────────────────────────┐
│                   Tauri 进程边界                      │
│                                                     │
│  前端 (Vue 3)              后端 (Rust)               │
│  ┌─────────────┐          ┌──────────────────────┐  │
│  │ 系统托盘     │◄────────►│ data_source::codex   │  │
│  │ 悬浮面板     │  IPC     │   sqlite_reader      │  │
│  │ 主控看板     │  invoke  │   jsonl_parser       │  │
│  │ 设置页       │          │   reconciler         │  │
│  └─────────────┘          │                      │  │
│                           │ data_source::claude  │  │
│                           │   (V1.0 扩展点)       │  │
│                           │                      │  │
│                           │ billing::matrix      │  │
│                           │ store::sqlite        │  │
│                           └──────────────────────┘  │
└─────────────────────────────────────────────────────┘
         │
         ▼
   ~/.codex/ (只读)  /  ~/.claude/ (V1.0，只读)
```

**核心原则**：
- Rust 后端做所有 I/O 和计算，前端只负责渲染
- `AgentSource` trait 从第一天起抽象，Codex 和 Claude Code 是两个实现
- 所有后端逻辑通过 Tauri `invoke` 命令暴露给前端，前端无直接文件访问权
- 解析结果写入 `~/.agent-prism/cache.db`（AgentPrism 自管理的 SQLite），前端查缓存

---

## 3. 数据层设计

### 3.1 后端模块结构

```
src-tauri/src/
├── data_source/
│   ├── mod.rs          # AgentSource trait 定义
│   ├── codex/
│   │   ├── mod.rs
│   │   ├── sqlite.rs   # 读取 state_*.sqlite → threads 表
│   │   ├── jsonl.rs    # 扫描 sessions/**/*.jsonl，取每 session 最后一条 token_count
│   │   └── reconciler.rs  # 双口径对账，计算差异率
│   └── claude/
│       └── mod.rs      # V1.0 占位，实现 AgentSource trait
├── billing/
│   └── matrix.rs       # 可配置价格表，估算费用
├── store/
│   └── db.rs           # AgentPrism 自己的 SQLite 缓存
└── commands.rs         # Tauri invoke 命令入口
```

### 3.2 AgentSource trait

```rust
pub trait AgentSource {
    fn name(&self) -> &str;
    fn discover(&self) -> Result<Vec<PathBuf>>;        // 自动发现数据文件
    fn threads(&self) -> Result<Vec<ThreadRecord>>;    // 返回统一结构
    fn sessions(&self) -> Result<Vec<SessionRecord>>;  // token 类型拆分
}
```

### 3.3 统一数据结构

```rust
pub struct ThreadRecord {
    pub id: String,
    pub title: String,
    pub cwd: String,
    pub model: String,
    pub model_provider: String,
    pub tokens_used: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source: String,   // "codex" | "claude"
}

pub struct SessionRecord {
    pub session_id: String,
    pub cwd: String,
    pub model_provider: String,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,
    pub total_tokens: i64,
    pub source: String,
}
```

### 3.4 JSONL 解析策略

- 每个 session 文件可能有多条 `token_count` 事件
- 只取每个 session 的**最后一条**有效 `token_count`，代表该 session 当前累计消耗
- 单条解析失败跳过，累计 warning 数量

### 3.5 缓存写入策略

- V0.1：手动触发刷新（`refresh` command）
- V0.5：后台定时轮询（每 30s 刷新缓存）

---

## 4. 计费矩阵

价格表内置于 App，用户可在设置页查看和修改。结构如下：

```json
{
  "codex-mini": {
    "input_per_1m": 1.5,
    "cached_input_per_1m": 0.375,
    "output_per_1m": 6.0
  },
  "gpt-4.1": {
    "input_per_1m": 2.0,
    "cached_input_per_1m": 0.5,
    "output_per_1m": 8.0
  }
}
```

费用公式：

```
费用 = uncached_input / 1M × 输入单价
     + cached_input  / 1M × 缓存输入单价
     + output        / 1M × 输出单价

uncached_input = input_tokens - cached_input_tokens
```

所有费用展示必须标注"估算，非真实账单"。

---

## 5. 里程碑功能边界

### V0.1 探针时代

**后端：**
- `codex::sqlite` — 读取 `~/.codex/state_*.sqlite`，解析 `threads` 表
- `codex::jsonl` — 扫描 `~/.codex/sessions/**/*.jsonl`，取每 session 最后一条 `token_count`
- `codex::reconciler` — 计算双口径差异率
- `billing::matrix` — 内置预设价格表，估算费用
- Tauri commands：`get_summary`、`get_threads`、`refresh`

**前端：**
- 系统托盘图标（静态）
- 点击托盘弹出悬浮面板：今日 token 总量、估算费用、最活跃项目
- 主窗口（双击托盘打开）：线程列表 + 对账状态

### V0.5 透视时代

**后端新增：**
- 后台定时轮询（每 30s 刷新缓存）
- 多维度聚合查询：`get_by_project`、`get_by_model`、`get_by_date`

**前端新增：**
- 主控看板：毛玻璃深色 UI（macOS `vibrancy` 效果）
- 圆环预算进度图（引入 echarts）
- 项目 / 模型 / 时间 三个维度视图
- Top N 排行榜

### V1.0 全视时代

**后端新增：**
- `claude::` 模块实现 `AgentSource` trait，读取 `~/.claude/` 数据
- 多 source 聚合查询，统一展示

**前端新增：**
- Source 切换筛选器
- 多 Agent 对比视图

---

## 6. 前端 UI 结构

### 6.1 窗口体系

```
系统托盘图标
    │
    ├── 单击 → 悬浮面板（无边框 tray window，macOS vibrancy）
    │            ├── 今日 token 总量（大字数字）
    │            ├── 估算费用（标注"估算"）
    │            ├── 最活跃项目
    │            └── "打开看板" 按钮
    │
    └── 双击 / 点击"打开看板" → 主窗口（独立 Tauri 窗口）
                 ├── V0.1：线程列表 + 对账状态
                 └── V0.5+：完整仪表盘
```

### 6.2 视觉规范

- 窗口材质：macOS `NSVisualEffectView`，通过 Tauri `vibrancy` 配置开启
- 配色：深色系，黑/白/高级灰为主
- 强调色：仅在模型区分和预算警告时使用（荧光蓝 `#4FC3F7` / 琥珀 `#FFB74D`）
- 字体：`-apple-system`，数字用大字号纤细字重
- 无边框：`decorations: false`

### 6.3 前端组件结构（V0.1）

```
src/
├── components/
│   ├── TrayPanel.vue      # 悬浮面板
│   └── ThreadList.vue     # 线程列表
├── views/
│   └── Dashboard.vue      # 主窗口视图
└── composables/
    └── useStats.ts        # 封装 invoke 调用
```

### 6.4 依赖规划

- `@tauri-apps/plugin-tray` — 系统托盘（V0.1 引入）
- `echarts` — 圆环图表（V0.5 引入）
- CSS：纯手写，不引入 UI 框架

---

## 7. 错误处理与数据可信边界

### 7.1 统一返回结构

```rust
#[derive(Serialize)]
pub struct CommandResult<T> {
    pub data: Option<T>,
    pub error: Option<String>,
    pub warnings: Vec<String>,
}
```

### 7.2 错误降级策略

| 场景 | 行为 |
|------|------|
| `~/.codex/` 目录不存在 | 前端显示"未检测到 Codex 数据"引导页 |
| SQLite 文件不存在 | 返回空数据 + warning，不报错 |
| JSONL 单条解析失败 | 跳过该条，累计 warning 数量 |
| 对账差异率 > 5% | UI 显示黄色警告提示 |

### 7.3 前端展示策略

- 有 warning：数据正常展示，面板底部显示"部分数据可能不完整"小提示
- 完全无数据：显示空状态引导页，不崩溃

### 7.4 数据可信标注

- Token 数量、会话数、时间范围、项目分布 → 标注"本机数据"
- 费用 → 始终标注"估算，非真实账单"

---

## 8. 测试策略

### 8.1 Rust 单元测试

```
src-tauri/tests/fixtures/
├── sample.sqlite
└── sessions/
    └── sample.jsonl
```

测试覆盖点：
- `sqlite.rs`：正常读取、空库、字段缺失
- `jsonl.rs`：多条 token_count 只取最后一条、格式异常跳过
- `reconciler.rs`：差异率计算、差异超阈值 warning
- `billing::matrix`：各模型费用计算公式正确性

### 8.2 前端测试

V0.1 阶段不写前端测试，手动验证。V0.5 引入图表后考虑快照测试。

### 8.3 集成验证

用本机真实的 `~/.codex/` 数据跑完整链路，与 Codex 官方统计数字人工对比，记录差异率。

---

## 9. 实现策略

采用**后端优先**方式：Rust 数据层先行，每个里程碑后端打通后前端跟进。Rust 逻辑通过 `cargo test` 独立验证，不依赖 UI 跑通。
