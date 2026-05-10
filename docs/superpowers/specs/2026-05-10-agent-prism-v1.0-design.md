# AgentPrism V1.0 设计文档：全视时代

**日期：** 2026-05-10  
**状态：** 已确认，待实现  
**目标：** 接入 Claude Code 数据源，实现双 Agent 完全解耦展示与切换

---

## 1. 背景与目标

V0.5 已实现 Codex 单数据源的多维可视化看板。V1.0 核心目标是：

1. 实现 `ClaudeCodeSource`，读取 `~/.claude/projects/**/*.jsonl` 中的消耗数据
2. 在 Dashboard 顶部标题位置引入 Agent 切换器，支持 Codex / Claude Code 完全解耦展示
3. 预算上限、计费价格表按 agent 隔离存储，切换 agent 时自动切换配置
4. 记住上次选择的 agent，默认显示 Claude Code

---

## 2. 已确认设计决策

| 决策点 | 结论 |
|--------|------|
| Agent 切换器位置 | 顶部标题区，完全替换 "AgentPrism" 文字，字号/字重与原标题一致 |
| 切换器形式 | 自定义 dropdown（显示当前 Agent 名 + ▾，展开后有 ✓ 标记） |
| 默认 Agent | Claude Code；记住上次选择，下次启动自动恢复 |
| 空状态处理 | 显示"未检测到 ~/.xxx 目录"提示，不禁用菜单项 |
| 价格表 | Codex、Claude Code 各一套，均可编辑，互不影响 |
| 存储隔离策略 | Store key 加 agent 前缀（方案 A） |
| Claude Code token 统计 | `cache_creation_input_tokens` 归入 `input_tokens` 一并按输入价格计费 |
| Command 架构 | 前端驱动：所有聚合 command 新增 `agent: String` 参数 |
| 旧数据迁移 | 首次启动时自动将旧 key 迁移到 `_codex` 后缀，幂等安全 |

---

## 3. 架构设计

### 3.1 整体分层

```
前端层
├── App.vue                  （路由：dashboard / settings，透传 currentAgent）
├── Dashboard.vue            （AgentSwitcher 替换原标题；按 agent 条件渲染 reconcile）
├── Settings.vue             （价格表/预算标题显示当前 agent；数据按 agent 加载）
└── components/
    └── AgentSwitcher.vue    （新增：自定义 dropdown 组件）

Composables
├── useAgentSwitch.ts        （新增：全局 agent 状态，初始化时读取 last_selected_agent）
├── useStats.ts              （改：loadSummary(agent)）
└── useAggregates.ts         （改：loadAll(agent)）

后端层
├── commands.rs              （所有聚合 command 新增 agent 参数）
├── data_source/
│   ├── mod.rs               （AgentSource trait，已有，无需改动）
│   ├── codex/               （已有，无需改动）
│   └── claude/              （新实现 ClaudeCodeSource）
├── billing/mod.rs           （新增 default_prices_claude_code()；原 default_prices() 改名为 default_prices_codex()）
└── store/mod.rs             （所有 get/set 方法加 agent 参数；新增 last_selected_agent；新增 migrate_legacy_keys）
```

### 3.2 数据流

```
用户切换 Agent
    ↓
useAgentSwitch.switchAgent(agent)
    ↓
invoke('set_last_selected_agent', { agent })   // 持久化
currentAgent.value = agent                     // 响应式更新
    ↓
Dashboard 监听 currentAgent 变化 → reload()
    ↓
invoke('get_summary', { agent })
invoke('get_by_project', { agent })
invoke('get_by_model', { agent })
invoke('get_by_date', { agent })
    ↓
后端按 agent 路由到对应 AgentSource
后端 store.get_budget_tokens(agent) / store.get_prices(agent)
    ↓
前端渲染对应数据
```

---

## 4. 后端设计

### 4.1 ClaudeCodeSource

**数据路径：** `~/.claude/projects/{encoded_path}/{session_id}.jsonl`

**Session 解析规则：**

每个 `.jsonl` 文件为一个 session，逐行读取：

1. 取第一条 `type == "user"` 消息，提取：
   - `cwd`：工作目录（原始路径）
   - `sessionId`：session 唯一 ID
   - `timestamp`：作为 `created_at`
2. 累加所有 `type == "assistant"` 消息的 `message.usage`：
   - `input_tokens += usage.input_tokens + usage.cache_creation_input_tokens`（cache_creation 按输入价计费，归入 input）
   - `cached_input_tokens += usage.cache_read_input_tokens`
   - `output_tokens += usage.output_tokens`
   - `total_tokens = input_tokens + cached_input_tokens + output_tokens`
3. 取最后一条 assistant 消息的 `message.model` 作为 session 模型
4. 取最后一条消息的 `timestamp` 作为 `updated_at`
5. 若无任何 assistant usage（空 session），跳过该文件，不产生 SessionRecord

**项目名提取（`encoded_path` → `cwd`）：**

`encoded_path` 规则：路径首 `/` 被省略，其余 `/` 替换为 `-`。反推：
```rust
fn decode_project_path(encoded: &str) -> String {
    "/".to_string() + &encoded.replace('-', "/")
    // "-Users-liang-repos-agent-prism" → "/Users/liang/repos/agent-prism"
}
```
注意：路径中本身含 `-` 的部分（如 `_fluxpress`）因使用 `_` 无歧义，不受影响。但路径中含 `-` 的目录名（如 `my-blog`）会被错误解码。因此仅用 `encoded_path` 的最后一段作为项目显示名，不做完整反推：
```rust
let display_name = encoded_path.split('-').last().unwrap_or(encoded_path);
// "-Users-liang-repos-agent-prism" → "agent-prism"
```

**SummaryData 结构（Claude Code）：**

Claude Code 无 SQLite 双口径，因此无 reconcile。共用同一个 `SummaryData` 结构体，但对 `reconcile` 字段使用 sentinel 值表示"不适用"：

```rust
// reconcile.diff_rate = -1.0 表示不适用
// 前端检查 diff_rate < 0 时不渲染对账区域
pub struct ReconcileResult {
    pub sqlite_total: i64,
    pub jsonl_total: i64,
    pub diff: i64,
    pub diff_rate: f64,       // -1.0 = N/A
    pub warning: Option<String>,
}
```

另外，Claude Code 无 `thread_count`（无 SQLite threads 表），`thread_count` 返回 0，前端对 Codex 显示"线程数"，对 Claude Code 不显示该指标。

### 4.2 BillingMatrix 扩展

```rust
impl BillingMatrix {
    // 原 default_prices() 改名
    pub fn default_prices_codex() -> IndexMap<String, ModelPrice> {
        // gpt-5.5, gpt-5.4, gpt-5.4-mini, gpt-5.2（已有）
    }

    // 新增
    pub fn default_prices_claude_code() -> IndexMap<String, ModelPrice> {
        // claude-opus-4-7:   input=15.0, cached=1.5,  output=75.0
        // claude-sonnet-4-6: input=3.0,  cached=0.3,  output=15.0
        // claude-haiku-4-5:  input=0.8,  cached=0.08, output=4.0
    }

    pub fn new_for_agent(agent: &str) -> Self {
        let prices = match agent {
            "claude-code" => Self::default_prices_claude_code(),
            _ => Self::default_prices_codex(),
        };
        Self { prices }
    }
}
```

原 `BillingMatrix::new()` 内部调用 `default_prices_codex()`，保持向后兼容。

### 4.3 Store 改造

**方法签名变更（所有涉及 agent 配置的方法加 `agent: &str` 参数）：**

```rust
// 预算
pub fn get_budget_tokens(&self, agent: &str) -> Result<Option<i64>>
pub fn set_budget_tokens(&self, agent: &str, tokens: i64) -> Result<()>

// 价格表
pub fn get_prices(&self, agent: &str) -> Result<Option<IndexMap<String, ModelPrice>>>
pub fn set_prices(&self, agent: &str, prices: &IndexMap<String, ModelPrice>) -> Result<()>
pub fn delete_prices(&self, agent: &str) -> Result<()>

// 上次选择的 agent（无 agent 前缀，全局唯一）
pub fn get_last_selected_agent(&self) -> Result<Option<String>>
pub fn set_last_selected_agent(&self, agent: &str) -> Result<()>

// 旧数据迁移
pub fn migrate_legacy_keys(&self) -> Result<()>
```

**Key 命名规则：**

```
budget_tokens_{agent}       例：budget_tokens_codex, budget_tokens_claude-code
custom_prices_{agent}       例：custom_prices_codex, custom_prices_claude-code
last_selected_agent         全局，无前缀
```

**迁移逻辑（在 `AppStore::new()` 中调用，幂等）：**

```sql
-- 迁移预算
INSERT OR IGNORE INTO meta (key, value)
    SELECT 'budget_tokens_codex', value FROM meta WHERE key = 'budget_tokens';
DELETE FROM meta WHERE key = 'budget_tokens';

-- 迁移价格表
INSERT OR IGNORE INTO meta (key, value)
    SELECT 'custom_prices_codex', value FROM meta WHERE key = 'custom_prices';
DELETE FROM meta WHERE key = 'custom_prices';
```

### 4.4 Commands 改造

所有涉及 agent 的 command 新增 `agent: String` 参数：

```rust
get_summary(agent: String)
get_threads(agent: String)       // 仅 Codex 有意义，Claude Code 返回空列表
get_by_project(agent: String)
get_by_model(agent: String)
get_by_date(agent: String)
get_budget(agent: String)
set_budget(agent: String, tokens: i64)
get_prices(agent: String)
set_prices(agent: String, prices: ...)
reset_prices(agent: String)
```

新增 command：

```rust
get_last_selected_agent()        // 返回 CommandResult<Option<String>>
set_last_selected_agent(agent: String)  // 返回 CommandResult<String>
```

内部路由逻辑（以 `get_summary` 为例）：

```rust
#[tauri::command]
pub fn get_summary(agent: String) -> CommandResult<SummaryData> {
    match agent.as_str() {
        "codex" => get_summary_codex(),
        "claude-code" => get_summary_claude_code(),
        _ => CommandResult::err(format!("未知 Agent: {agent}")),
    }
}
```

---

## 5. 前端设计

### 5.1 useAgentSwitch composable（新增）

```typescript
// src/composables/useAgentSwitch.ts
export type AgentId = 'codex' | 'claude-code'

export interface AgentInfo {
  id: AgentId
  label: string          // 显示名：'Claude Code' | 'Codex'
}

export const AGENTS: AgentInfo[] = [
  { id: 'claude-code', label: 'Claude Code' },
  { id: 'codex',       label: 'Codex' },
]

export function useAgentSwitch() {
  const currentAgent = ref<AgentId>('claude-code')

  // 初始化时从后端读取上次选择
  async function init() {
    const res = await invoke<CommandResult<string | null>>('get_last_selected_agent')
    if (res.data && (res.data === 'codex' || res.data === 'claude-code')) {
      currentAgent.value = res.data
    }
  }

  async function switchAgent(agent: AgentId) {
    if (currentAgent.value === agent) return
    currentAgent.value = agent
    await invoke('set_last_selected_agent', { agent })
  }

  return { currentAgent, init, switchAgent, AGENTS }
}
```

### 5.2 AgentSwitcher 组件（新增）

```
// src/components/AgentSwitcher.vue
// 样式与原 .logo 完全一致（font-size: 14px; font-weight: 500; letter-spacing: 0.08em）
// 点击展开自定义 dropdown，点击外部收起
// 当前选中项前显示 ✓
```

交互细节：
- Dropdown 向下展开，宽度自适应内容
- 背景白色，边框 1px solid #e0e0e0，圆角 6px，轻阴影
- 菜单项 hover 时背景 #f5f5f5
- 切换后立即收起，Dashboard 进入 loading 状态

### 5.3 Dashboard.vue 改造

1. 引入 `useAgentSwitch`，替换顶部 `<span class="logo">AgentPrism</span>` 为 `<AgentSwitcher>`
2. 监听 `currentAgent` 变化，触发 `reload(agent)`
3. 所有 invoke 调用传入 `currentAgent.value`：
   ```typescript
   await Promise.all([loadSummary(agent), loadAll(agent)])
   await invoke('get_budget', { agent })
   ```
4. 概览区条件渲染：
   - `thread_count`：仅 `agent === 'codex'` 时显示
   - reconcile 差异率：仅 `agent === 'codex'` 且 `diff_rate >= 0` 时显示
5. 空状态文案按 agent 动态：
   - codex：`未检测到 ~/.codex 目录`
   - claude-code：`未检测到 ~/.claude 目录`

### 5.4 Settings.vue 改造

1. 接收 `currentAgent` prop（从 App.vue 透传）
2. 价格表区域标题：`计费价格表 · {{ agentLabel }}（/1M token，单位：$）`
3. 所有价格表/预算 invoke 调用传入 `agent`：
   ```typescript
   invoke('get_prices', { agent: props.currentAgent })
   invoke('set_prices', { agent: props.currentAgent, prices: prices.value })
   invoke('reset_prices', { agent: props.currentAgent })
   invoke('get_budget', { agent: props.currentAgent })
   invoke('set_budget', { agent: props.currentAgent, tokens: val })
   ```
4. 当 `currentAgent` prop 变化时（实际上不会，因为 Settings 页面只能从 Dashboard 进入，agent 已固定），重新加载配置

### 5.5 App.vue 改造

```typescript
// 在 App.vue 中初始化 useAgentSwitch，向下透传 currentAgent
const { currentAgent, init } = useAgentSwitch()
onMounted(() => init())
```

```html
<Dashboard v-if="page === 'dashboard'"
  :currentAgent="currentAgent"
  @openSettings="page = 'settings'" />
<Settings v-else
  :currentAgent="currentAgent"
  @back="page = 'dashboard'" />
```

---

## 6. 错误处理

| 场景 | 处理方式 |
|------|---------|
| `~/.codex` 不存在 | 返回 `CommandResult::err`，前端显示空状态提示 |
| `~/.claude` 不存在 | 同上 |
| JSONL 文件解析异常 | 跳过该文件，加入 warnings，正常返回其他数据 |
| 未知 agent 参数 | 后端返回 `CommandResult::err("未知 Agent: {agent}")` |
| 旧数据迁移失败 | 记录错误日志，继续启动（不阻断）|

---

## 7. 测试策略

### 后端单元测试

- `ClaudeCodeSource::sessions()`：使用 fixture JSONL 文件测试 token 累加逻辑
- `decode_project_path()`：测试路径解码边界情况
- `store.migrate_legacy_keys()`：测试迁移幂等性
- `BillingMatrix::default_prices_claude_code()`：验证价格表结构

### 集成验证（手动）

- 切换 Agent：概览数字、Tab 内容、价格表全部切换
- 关闭重开：自动恢复上次选择的 agent
- Codex 不存在时：切换到 Codex 显示空状态，Claude Code 仍正常
- 价格表独立编辑：修改 Claude Code 价格表不影响 Codex

---

## 8. 不在 V1.0 范围内

- Agent 数量超过 2 个
- 跨 Agent 数据汇总视图
- 价格表从云端自动更新
- `get_threads` 对 Claude Code 的有意义实现（V1.0 返回空列表）
