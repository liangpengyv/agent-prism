# AgentPrism V0.5 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 AgentPrism V0.5 透视时代——上线主控看板，完成项目/模型/时间三个维度的数据折射与可视化，引入圆环预算图，实现后台定时轮询与前端自动更新。

**Architecture:** 后端新增三个多维聚合 Tauri command + 后台定时轮询线程；前端重构 Dashboard 为三 Tab 看板，引入 ECharts 圆环图/柱状图/折线图，新增轻量 Settings 页，以 ref 驱动页面切换。

**Tech Stack:** Tauri 2, Vue 3, Rust, ECharts, vue-echarts

**当前状态（V0.1 已完成）：**
- Rust 后端：`data_source::codex`（sqlite/jsonl/reconciler）、`billing::matrix`、`store`、3 个 Tauri commands（get_summary/get_threads/refresh）
- 前端：Dashboard（原生标题栏、浅色 UI）、ThreadList、TrayPanel、useStats composable
- 系统托盘：左键切换窗口显隐、右键菜单

---

## 文件结构

**新建文件：**
- `src-tauri/src/commands/mod.rs` — 拆分 commands 为模块
- `src-tauri/src/commands/aggregates.rs` — 三个聚合查询 command
- `src/components/BudgetRing.vue` — 圆环预算图组件
- `src/components/ProjectList.vue` — 项目维度排行榜
- `src/components/ModelBreakdown.vue` — 模型维度饼图
- `src/components/DayChart.vue` — 时间维度折线图
- `src/views/Settings.vue` — 设置页（预算上限配置）
- `src/composables/useAggregates.ts` — 封装聚合 invoke 调用

**修改文件：**
- `src-tauri/src/commands.rs` — 新增聚合 command 及注册，添加 data-updated 事件推送
- `src-tauri/src/lib.rs` — 添加后台定时轮询线程（每 30s），注册新 commands
- `src-tauri/src/store/mod.rs` — 新增 budget_tokens 配置读写
- `src/views/Dashboard.vue` — 重构为三 Tab 看板 + BudgetRing
- `src/App.vue` — 添加页面切换（Dashboard ↔ Settings）
- `src/composables/useStats.ts` — 添加监听 data-updated 事件
- `package.json` — 添加 echarts、vue-echarts 依赖

---

## Task 1: 添加 ECharts 前端依赖

**Files:**
- Modify: `package.json`

- [ ] **Step 1: 安装依赖**

```bash
pnpm add echarts vue-echarts
```

- [ ] **Step 2: 验证安装**

```bash
pnpm list echarts vue-echarts
```

预期：两个包均出现在依赖列表中

- [ ] **Step 3: 提交**

```bash
git add package.json pnpm-lock.yaml
git commit -m "chore: 添加 echarts 和 vue-echarts 依赖"
```

---

## Task 2: 新增后端多维聚合数据结构和 commands

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

新增三个 Tauri command：`get_by_project`、`get_by_model`、`get_by_date`

- [ ] **Step 1: 编写聚合测试**

在 `src-tauri/src/commands.rs` 中添加三个聚合数据结构和对应 command（先写结构，todo! 占位实现）：

```rust
// 在 commands.rs 末尾追加

#[derive(Serialize, Clone)]
pub struct ProjectStat {
    pub project: String,
    pub tokens: i64,
    pub cost_usd: f64,
}

#[derive(Serialize, Clone)]
pub struct ModelStat {
    pub model: String,
    pub tokens: i64,
    pub cost_usd: f64,
}

#[derive(Serialize, Clone)]
pub struct DayStat {
    pub date: String,   // "YYYY-MM-DD"
    pub tokens: i64,
    pub cost_usd: f64,
}
```

- [ ] **Step 2: 实现 get_by_project**

从 `CodexSource::threads()` 按 `cwd` 聚合 token，再用 `BillingMatrix` 从 sessions 中计算各 cwd 的费用（sessions 无 cwd 时用 threads 估算）：

```rust
#[tauri::command]
pub fn get_by_project() -> CommandResult<Vec<ProjectStat>> {
    let source = match CodexSource::new() {
        Some(s) => s,
        None => return CommandResult::err("未检测到 ~/.codex 目录"),
    };

    let (threads, warnings) = match source.threads() {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("读取 threads 失败: {e}")),
    };

    let mut map: std::collections::HashMap<String, i64> = Default::default();
    for t in &threads {
        let key = t.cwd.split('/').last().unwrap_or(&t.cwd).to_string();
        *map.entry(key).or_insert(0) += t.tokens_used;
    }

    // 简单用 codex-mini 单价估算（不区分模型）
    let input_price = 1.5_f64;
    let output_price = 6.0_f64;
    let avg_price = (input_price + output_price) / 2.0 / 1_000_000.0;

    let mut stats: Vec<ProjectStat> = map
        .into_iter()
        .map(|(project, tokens)| ProjectStat {
            cost_usd: tokens as f64 * avg_price,
            project,
            tokens,
        })
        .collect();
    stats.sort_by(|a, b| b.tokens.cmp(&a.tokens));

    CommandResult::ok_with_warnings(stats, warnings)
}
```

- [ ] **Step 3: 实现 get_by_model**

从 `CodexSource::sessions()` 按 `model_provider` 聚合 token 和费用：

```rust
#[tauri::command]
pub fn get_by_model() -> CommandResult<Vec<ModelStat>> {
    let source = match CodexSource::new() {
        Some(s) => s,
        None => return CommandResult::err("未检测到 ~/.codex 目录"),
    };

    let (sessions, warnings) = match source.sessions() {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("读取 sessions 失败: {e}")),
    };

    let matrix = BillingMatrix::new();

    let mut token_map: std::collections::HashMap<String, i64> = Default::default();
    let mut cost_map: std::collections::HashMap<String, f64> = Default::default();

    for s in &sessions {
        *token_map.entry(s.model_provider.clone()).or_insert(0) += s.total_tokens;
        let cost = matrix.estimate(std::slice::from_ref(s)).total_usd;
        *cost_map.entry(s.model_provider.clone()).or_insert(0.0) += cost;
    }

    let mut stats: Vec<ModelStat> = token_map
        .into_iter()
        .map(|(model, tokens)| ModelStat {
            cost_usd: *cost_map.get(&model).unwrap_or(&0.0),
            model,
            tokens,
        })
        .collect();
    stats.sort_by(|a, b| b.tokens.cmp(&a.tokens));

    CommandResult::ok_with_warnings(stats, warnings)
}
```

- [ ] **Step 4: 实现 get_by_date**

从 `CodexSource::threads()` 按 `updated_at` 的 UTC 日期聚合近 30 天数据：

```rust
#[tauri::command]
pub fn get_by_date() -> CommandResult<Vec<DayStat>> {
    use chrono::{Duration, Utc, Datelike};

    let source = match CodexSource::new() {
        Some(s) => s,
        None => return CommandResult::err("未检测到 ~/.codex 目录"),
    };

    let (threads, warnings) = match source.threads() {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("读取 threads 失败: {e}")),
    };

    let cutoff = Utc::now() - Duration::days(30);
    let input_price = 1.5_f64;
    let output_price = 6.0_f64;
    let avg_price = (input_price + output_price) / 2.0 / 1_000_000.0;

    let mut map: std::collections::BTreeMap<String, i64> = Default::default();
    for t in &threads {
        if t.updated_at < cutoff { continue; }
        let date = format!("{:04}-{:02}-{:02}",
            t.updated_at.year(), t.updated_at.month(), t.updated_at.day());
        *map.entry(date).or_insert(0) += t.tokens_used;
    }

    let stats: Vec<DayStat> = map
        .into_iter()
        .map(|(date, tokens)| DayStat {
            cost_usd: tokens as f64 * avg_price,
            date,
            tokens,
        })
        .collect();

    CommandResult::ok_with_warnings(stats, warnings)
}
```

- [ ] **Step 5: 在 lib.rs 注册新 commands**

在 `src-tauri/src/lib.rs` 的 `invoke_handler` 中追加三个新 command：

```rust
.invoke_handler(tauri::generate_handler![
    get_summary, get_threads, refresh,
    get_by_project, get_by_model, get_by_date
])
```

同时在 `use commands::` 中添加三个新名称：

```rust
use commands::{get_summary, get_threads, refresh, get_by_project, get_by_model, get_by_date};
```

- [ ] **Step 6: 编译验证**

```bash
cd src-tauri && cargo build 2>&1 | tail -20
```

预期：编译成功，无错误

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: 新增 get_by_project/get_by_model/get_by_date 聚合 command"
```

---

## Task 3: 后台定时轮询与 data-updated 事件

**Files:**
- Modify: `src-tauri/src/lib.rs`

在 `setup` 回调中启动一个后台线程，每 30s 推送 `data-updated` 事件给前端。

- [ ] **Step 1: 添加后台轮询线程**

在 `lib.rs` 的 `setup` 闭包中（`TrayIconBuilder` 之前）添加：

```rust
// 后台定时轮询：每 30s 推送 data-updated 事件
let app_handle = app.handle().clone();
std::thread::spawn(move || {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(30));
        let _ = app_handle.emit("data-updated", ());
    }
});
```

需要在文件顶部引入 `use tauri::Emitter;`

- [ ] **Step 2: 编译验证**

```bash
cd src-tauri && cargo build 2>&1 | tail -20
```

预期：编译成功

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: 后台线程每 30s 推送 data-updated 事件"
```

---

## Task 4: store 支持预算配置读写

**Files:**
- Modify: `src-tauri/src/store/mod.rs`
- Modify: `src-tauri/src/commands.rs`

新增 `get_budget` / `set_budget` command，将预算上限（token 数量）存入 `~/.agent-prism/cache.db` 的 `meta` 表。

- [ ] **Step 1: 扩展 AppStore 方法**

在 `src-tauri/src/store/mod.rs` 中追加：

```rust
pub fn set_budget_tokens(&self, tokens: i64) -> Result<()> {
    let conn = Connection::open(&self.db_path)?;
    conn.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES ('budget_tokens', ?1)",
        params![tokens.to_string()],
    )?;
    Ok(())
}

pub fn get_budget_tokens(&self) -> Result<Option<i64>> {
    let conn = Connection::open(&self.db_path)?;
    let result: rusqlite::Result<String> = conn.query_row(
        "SELECT value FROM meta WHERE key = 'budget_tokens'",
        [],
        |row| row.get(0),
    );
    match result {
        Ok(s) => Ok(s.parse().ok()),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
```

- [ ] **Step 2: 新增 get_budget / set_budget command**

在 `src-tauri/src/commands.rs` 中追加：

```rust
#[tauri::command]
pub fn get_budget() -> CommandResult<Option<i64>> {
    use crate::store::AppStore;
    match AppStore::new() {
        Ok(store) => match store.get_budget_tokens() {
            Ok(v) => CommandResult::ok(v),
            Err(e) => CommandResult::err(format!("读取预算失败: {e}")),
        },
        Err(e) => CommandResult::err(format!("初始化 store 失败: {e}")),
    }
}

#[tauri::command]
pub fn set_budget(tokens: i64) -> CommandResult<String> {
    use crate::store::AppStore;
    match AppStore::new() {
        Ok(store) => match store.set_budget_tokens(tokens) {
            Ok(_) => CommandResult::ok("预算已保存".to_string()),
            Err(e) => CommandResult::err(format!("保存预算失败: {e}")),
        },
        Err(e) => CommandResult::err(format!("初始化 store 失败: {e}")),
    }
}
```

- [ ] **Step 3: 在 lib.rs 注册**

```rust
use commands::{..., get_budget, set_budget};
// invoke_handler 追加
get_budget, set_budget
```

- [ ] **Step 4: 编译验证**

```bash
cd src-tauri && cargo build 2>&1 | tail -20
```

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/store/mod.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: store 支持预算 token 上限读写，新增 get_budget/set_budget command"
```

---

## Task 5: 前端 useAggregates composable

**Files:**
- Create: `src/composables/useAggregates.ts`
- Modify: `src/composables/useStats.ts`

- [ ] **Step 1: 实现 useAggregates.ts**

```typescript
// src/composables/useAggregates.ts
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CommandResult } from './useStats'

export interface ProjectStat {
  project: string
  tokens: number
  cost_usd: number
}

export interface ModelStat {
  model: string
  tokens: number
  cost_usd: number
}

export interface DayStat {
  date: string
  tokens: number
  cost_usd: number
}

export function useAggregates() {
  const byProject = ref<ProjectStat[]>([])
  const byModel = ref<ModelStat[]>([])
  const byDate = ref<DayStat[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function loadAll() {
    loading.value = true
    error.value = null
    try {
      const [pRes, mRes, dRes] = await Promise.all([
        invoke<CommandResult<ProjectStat[]>>('get_by_project'),
        invoke<CommandResult<ModelStat[]>>('get_by_model'),
        invoke<CommandResult<DayStat[]>>('get_by_date'),
      ])
      if (pRes.error) error.value = pRes.error
      else byProject.value = pRes.data ?? []
      if (!mRes.error) byModel.value = mRes.data ?? []
      if (!dRes.error) byDate.value = dRes.data ?? []
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  return { byProject, byModel, byDate, loading, error, loadAll }
}
```

- [ ] **Step 2: 在 useStats.ts 中添加 data-updated 事件监听**

在 `useStats.ts` 末尾的 `return` 之前添加：

```typescript
import { listen } from '@tauri-apps/api/event'

export function useDataUpdatedListener(callback: () => void) {
  let unlisten: (() => void) | null = null
  listen('data-updated', () => callback()).then(fn => { unlisten = fn })
  return () => { if (unlisten) unlisten() }
}
```

- [ ] **Step 3: 提交**

```bash
git add src/composables/useAggregates.ts src/composables/useStats.ts
git commit -m "feat: 实现 useAggregates composable，添加 data-updated 事件监听工具"
```

---

## Task 6: BudgetRing 圆环图组件

**Files:**
- Create: `src/components/BudgetRing.vue`

显示 token 消耗相对预算上限的进度圆环。

- [ ] **Step 1: 实现 BudgetRing.vue**

```vue
<!-- src/components/BudgetRing.vue -->
<script setup lang="ts">
import { computed } from 'vue'
import { use } from 'echarts/core'
import { GaugeChart } from 'echarts/charts'
import { CanvasRenderer } from 'echarts/renderers'
import VChart from 'vue-echarts'

use([GaugeChart, CanvasRenderer])

const props = defineProps<{
  usedTokens: number
  budgetTokens: number
}>()

const percent = computed(() =>
  props.budgetTokens > 0
    ? Math.min(100, Math.round((props.usedTokens / props.budgetTokens) * 100))
    : 0
)

const color = computed(() =>
  percent.value >= 90 ? '#FFB74D' : '#4FC3F7'
)

const option = computed(() => ({
  series: [{
    type: 'gauge',
    startAngle: 210,
    endAngle: -30,
    min: 0,
    max: 100,
    splitNumber: 0,
    radius: '88%',
    axisLine: {
      lineStyle: {
        width: 10,
        color: [[percent.value / 100, color.value], [1, 'rgba(255,255,255,0.08)']],
      },
    },
    axisTick: { show: false },
    splitLine: { show: false },
    axisLabel: { show: false },
    pointer: { show: false },
    detail: {
      valueAnimation: true,
      formatter: '{value}%',
      color: color.value,
      fontSize: 20,
      fontWeight: 300,
      offsetCenter: [0, '10%'],
    },
    data: [{ value: percent.value }],
  }],
}))
</script>

<template>
  <div class="budget-ring">
    <VChart :option="option" autoresize style="width:140px;height:140px;" />
    <div class="ring-label">预算消耗</div>
  </div>
</template>

<style scoped>
.budget-ring {
  display: flex;
  flex-direction: column;
  align-items: center;
}
.ring-label {
  font-size: 10px;
  color: #888;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  margin-top: -8px;
}
</style>
```

- [ ] **Step 2: 提交**

```bash
git add src/components/BudgetRing.vue
git commit -m "feat: 实现 BudgetRing 圆环预算图组件"
```

---

## Task 7: 项目/模型/时间维度子组件

**Files:**
- Create: `src/components/ProjectList.vue`
- Create: `src/components/ModelBreakdown.vue`
- Create: `src/components/DayChart.vue`

- [ ] **Step 1: 实现 ProjectList.vue**

```vue
<!-- src/components/ProjectList.vue -->
<script setup lang="ts">
import type { ProjectStat } from '../composables/useAggregates'

const props = defineProps<{ stats: ProjectStat[] }>()

const maxTokens = computed(() =>
  props.stats.length > 0 ? props.stats[0].tokens : 1
)

import { computed } from 'vue'

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return String(n)
}
</script>

<template>
  <div class="project-list">
    <div v-if="stats.length === 0" class="empty">暂无项目数据</div>
    <div v-for="(s, i) in stats" :key="s.project" class="project-row">
      <div class="rank">{{ i + 1 }}</div>
      <div class="info">
        <div class="name">{{ s.project }}</div>
        <div class="bar-wrap">
          <div class="bar" :style="{ width: (s.tokens / maxTokens * 100) + '%' }"></div>
        </div>
      </div>
      <div class="right">
        <div class="tokens">{{ formatTokens(s.tokens) }}</div>
        <div class="cost">${{ s.cost_usd.toFixed(3) }}</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.project-list { display: flex; flex-direction: column; gap: 8px; padding: 4px 0; }
.empty { color: #aaa; font-size: 13px; text-align: center; padding: 20px; }
.project-row { display: flex; align-items: center; gap: 10px; padding: 6px 0; border-bottom: 1px solid #f0f0f0; }
.rank { font-size: 12px; color: #bbb; width: 18px; text-align: center; flex-shrink: 0; }
.info { flex: 1; min-width: 0; }
.name { font-size: 13px; color: #333; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-bottom: 4px; }
.bar-wrap { height: 3px; background: #f0f0f0; border-radius: 2px; overflow: hidden; }
.bar { height: 100%; background: #4FC3F7; border-radius: 2px; transition: width 0.3s; }
.right { text-align: right; flex-shrink: 0; }
.tokens { font-size: 13px; color: #333; }
.cost { font-size: 11px; color: #888; }
</style>
```

- [ ] **Step 2: 实现 ModelBreakdown.vue**

```vue
<!-- src/components/ModelBreakdown.vue -->
<script setup lang="ts">
import { computed } from 'vue'
import { use } from 'echarts/core'
import { PieChart } from 'echarts/charts'
import { TooltipComponent, LegendComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import VChart from 'vue-echarts'
import type { ModelStat } from '../composables/useAggregates'

use([PieChart, TooltipComponent, LegendComponent, CanvasRenderer])

const props = defineProps<{ stats: ModelStat[] }>()

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return String(n)
}

const COLORS = ['#4FC3F7', '#FFB74D', '#81C784', '#CE93D8', '#F48FB1']

const option = computed(() => ({
  tooltip: { trigger: 'item', formatter: (p: any) => `${p.name}: ${formatTokens(p.value)} tokens` },
  color: COLORS,
  series: [{
    type: 'pie',
    radius: ['45%', '72%'],
    data: props.stats.map(s => ({ name: s.model || '未知', value: s.tokens })),
    label: { show: true, formatter: '{b}\n{d}%', fontSize: 11, color: '#555' },
    emphasis: { itemStyle: { shadowBlur: 6, shadowColor: 'rgba(0,0,0,0.1)' } },
  }],
}))
</script>

<template>
  <div class="model-breakdown">
    <div v-if="stats.length === 0" class="empty">暂无模型数据</div>
    <VChart v-else :option="option" autoresize style="width:100%;height:220px;" />
  </div>
</template>

<style scoped>
.model-breakdown { width: 100%; }
.empty { color: #aaa; font-size: 13px; text-align: center; padding: 40px; }
</style>
```

- [ ] **Step 3: 实现 DayChart.vue**

```vue
<!-- src/components/DayChart.vue -->
<script setup lang="ts">
import { computed } from 'vue'
import { use } from 'echarts/core'
import { BarChart } from 'echarts/charts'
import { GridComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import VChart from 'vue-echarts'
import type { DayStat } from '../composables/useAggregates'

use([BarChart, GridComponent, TooltipComponent, CanvasRenderer])

const props = defineProps<{ stats: DayStat[] }>()

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return String(n)
}

const option = computed(() => ({
  tooltip: {
    trigger: 'axis',
    formatter: (params: any[]) => {
      const p = params[0]
      return `${p.axisValue}<br/>${formatTokens(p.value)} tokens`
    },
  },
  grid: { left: 40, right: 12, top: 12, bottom: 40 },
  xAxis: {
    type: 'category',
    data: props.stats.map(d => d.date.slice(5)),
    axisLabel: { fontSize: 10, color: '#aaa', rotate: 45 },
    axisLine: { lineStyle: { color: '#e0e0e0' } },
  },
  yAxis: {
    type: 'value',
    axisLabel: { formatter: (v: number) => formatTokens(v), fontSize: 10, color: '#aaa' },
    splitLine: { lineStyle: { color: '#f5f5f5' } },
  },
  series: [{
    type: 'bar',
    data: props.stats.map(d => d.tokens),
    itemStyle: { color: '#4FC3F7', borderRadius: [2, 2, 0, 0] },
    barMaxWidth: 24,
  }],
}))
</script>

<template>
  <div class="day-chart">
    <div v-if="stats.length === 0" class="empty">近 30 天暂无数据</div>
    <VChart v-else :option="option" autoresize style="width:100%;height:220px;" />
  </div>
</template>

<style scoped>
.day-chart { width: 100%; }
.empty { color: #aaa; font-size: 13px; text-align: center; padding: 40px; }
</style>
```

- [ ] **Step 4: 提交**

```bash
git add src/components/ProjectList.vue src/components/ModelBreakdown.vue src/components/DayChart.vue
git commit -m "feat: 实现项目/模型/时间三个维度可视化组件"
```

---

## Task 8: 设置页 Settings.vue

**Files:**
- Create: `src/views/Settings.vue`

轻量设置页，仅包含月度 Token 预算上限输入框。

- [ ] **Step 1: 实现 Settings.vue**

```vue
<!-- src/views/Settings.vue -->
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CommandResult } from '../composables/useStats'

const emit = defineEmits<{ back: [] }>()

const DEFAULT_BUDGET = 10_000_000

const budgetInput = ref<string>('')
const saving = ref(false)
const saveMsg = ref<string | null>(null)

onMounted(async () => {
  const res = await invoke<CommandResult<number | null>>('get_budget')
  budgetInput.value = String(res.data ?? DEFAULT_BUDGET)
})

async function save() {
  const val = parseInt(budgetInput.value, 10)
  if (isNaN(val) || val <= 0) return
  saving.value = true
  saveMsg.value = null
  try {
    await invoke('set_budget', { tokens: val })
    saveMsg.value = '已保存'
    setTimeout(() => { saveMsg.value = null }, 2000)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="settings">
    <header class="header">
      <button class="back-btn" @click="$emit('back')">← 返回</button>
      <span class="title">设置</span>
    </header>

    <div class="section">
      <div class="section-title">预算管理</div>
      <div class="field">
        <label class="field-label">月度 Token 预算上限</label>
        <div class="field-row">
          <input
            v-model="budgetInput"
            type="number"
            min="1"
            class="field-input"
            placeholder="例：10000000"
          />
          <button class="save-btn" @click="save" :disabled="saving">
            {{ saving ? '保存中…' : '保存' }}
          </button>
        </div>
        <div class="field-hint">用于圆环预算图的上限基准（单位：token）</div>
        <div v-if="saveMsg" class="save-msg">{{ saveMsg }}</div>
      </div>
    </div>

    <div class="section">
      <div class="section-title">计费价格表（内置，仅供参考）</div>
      <table class="price-table">
        <thead>
          <tr><th>模型</th><th>输入 /1M</th><th>缓存输入 /1M</th><th>输出 /1M</th></tr>
        </thead>
        <tbody>
          <tr><td>codex-mini</td><td>$1.50</td><td>$0.375</td><td>$6.00</td></tr>
          <tr><td>gpt-4.1</td><td>$2.00</td><td>$0.50</td><td>$8.00</td></tr>
          <tr><td>gpt-4.1-mini</td><td>$0.40</td><td>$0.10</td><td>$1.60</td></tr>
        </tbody>
      </table>
      <div class="price-note">所有费用均为估算，非真实账单</div>
    </div>
  </div>
</template>

<style scoped>
.settings { display: flex; flex-direction: column; height: 100vh; font-family: -apple-system, sans-serif; color: #333; }
.header { display: flex; align-items: center; gap: 12px; padding: 10px 20px; border-bottom: 1px solid #e0e0e0; }
.back-btn { background: none; border: none; color: #0077cc; font-size: 13px; cursor: pointer; padding: 0; }
.back-btn:hover { text-decoration: underline; }
.title { font-size: 14px; font-weight: 500; }
.section { padding: 20px; border-bottom: 1px solid #f0f0f0; }
.section-title { font-size: 11px; color: #888; text-transform: uppercase; letter-spacing: 0.06em; margin-bottom: 14px; }
.field-label { font-size: 13px; color: #333; display: block; margin-bottom: 8px; }
.field-row { display: flex; gap: 8px; align-items: center; }
.field-input { flex: 1; padding: 6px 10px; border: 1px solid #ccc; border-radius: 5px; font-size: 13px; color: #333; }
.field-hint { font-size: 11px; color: #aaa; margin-top: 6px; }
.save-btn { background: #0077cc; border: none; border-radius: 5px; color: #fff; font-size: 12px; padding: 6px 14px; cursor: pointer; }
.save-btn:hover { background: #005fa3; }
.save-btn:disabled { opacity: 0.5; }
.save-msg { font-size: 12px; color: #4CAF50; margin-top: 6px; }
.price-table { width: 100%; border-collapse: collapse; font-size: 12px; }
.price-table th { text-align: left; padding: 6px 8px; color: #888; font-weight: 400; border-bottom: 1px solid #e0e0e0; }
.price-table td { padding: 6px 8px; border-bottom: 1px solid #f5f5f5; color: #333; }
.price-note { font-size: 11px; color: #aaa; margin-top: 10px; }
</style>
```

- [ ] **Step 2: 提交**

```bash
git add src/views/Settings.vue
git commit -m "feat: 实现设置页（预算配置 + 计费价格表展示）"
```

---

## Task 9: 重构 Dashboard 为三 Tab 看板

**Files:**
- Modify: `src/views/Dashboard.vue`

将现有线程列表 Dashboard 升级为带 BudgetRing + 三 Tab 切换的完整看板。

- [ ] **Step 1: 重写 Dashboard.vue**

```vue
<!-- src/views/Dashboard.vue -->
<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useStats, useDataUpdatedListener } from '../composables/useStats'
import { useAggregates } from '../composables/useAggregates'
import BudgetRing from '../components/BudgetRing.vue'
import ProjectList from '../components/ProjectList.vue'
import ModelBreakdown from '../components/ModelBreakdown.vue'
import DayChart from '../components/DayChart.vue'
import type { CommandResult } from '../composables/useStats'

const emit = defineEmits<{ openSettings: [] }>()

const { summary, error, loading, loadSummary } = useStats()
const { byProject, byModel, byDate, loadAll } = useAggregates()
const activeTab = ref<'project' | 'model' | 'date'>('project')
const budgetTokens = ref(10_000_000)

async function loadBudget() {
  const res = await invoke<CommandResult<number | null>>('get_budget')
  if (res.data != null) budgetTokens.value = res.data
}

async function reload() {
  await Promise.all([loadSummary(), loadAll()])
}

onMounted(async () => {
  await Promise.all([reload(), loadBudget()])
})

const stopListen = useDataUpdatedListener(() => reload())
onUnmounted(() => stopListen())

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return String(n)
}
</script>

<template>
  <div class="dashboard">
    <header class="header">
      <span class="logo">AgentPrism</span>
      <div class="header-actions">
        <button class="action-btn" @click="reload" :disabled="loading">
          {{ loading ? '刷新中…' : '刷新' }}
        </button>
        <button class="action-btn" @click="$emit('openSettings')">设置</button>
      </div>
    </header>

    <div v-if="error" class="error-state">{{ error }}</div>

    <template v-else>
      <!-- 概览区 -->
      <div class="overview" v-if="summary">
        <BudgetRing
          :usedTokens="summary.total_tokens"
          :budgetTokens="budgetTokens"
        />
        <div class="stats-grid">
          <div class="stat">
            <div class="stat-value">{{ formatTokens(summary.total_tokens) }}</div>
            <div class="stat-label">Token 总量</div>
          </div>
          <div class="stat">
            <div class="stat-value accent">${{ summary.estimated_cost_usd.toFixed(4) }}</div>
            <div class="stat-label">估算费用</div>
          </div>
          <div class="stat">
            <div class="stat-value">{{ summary.thread_count }}</div>
            <div class="stat-label">线程数</div>
          </div>
          <div class="stat">
            <div class="stat-value">{{ summary.session_count }}</div>
            <div class="stat-label">Session 数</div>
          </div>
          <div class="reconcile" :class="{ warn: summary.reconcile.warning }">
            对账差异率 {{ (summary.reconcile.diff_rate * 100).toFixed(1) }}%
          </div>
        </div>
      </div>

      <!-- Tab 切换 -->
      <div class="tabs">
        <button
          v-for="t in [['project','项目'],['model','模型'],['date','时间']] as const"
          :key="t[0]"
          class="tab-btn"
          :class="{ active: activeTab === t[0] }"
          @click="activeTab = t[0]"
        >{{ t[1] }}</button>
      </div>

      <!-- Tab 内容区 -->
      <div class="tab-content">
        <ProjectList v-if="activeTab === 'project'" :stats="byProject" />
        <ModelBreakdown v-else-if="activeTab === 'model'" :stats="byModel" />
        <DayChart v-else :stats="byDate" />
      </div>

      <div class="estimate-footer">估算，非真实账单</div>
    </template>
  </div>
</template>

<style scoped>
.dashboard { display: flex; flex-direction: column; height: 100vh; font-family: -apple-system, sans-serif; color: #333; overflow: hidden; }
.header { display: flex; justify-content: space-between; align-items: center; padding: 10px 20px; border-bottom: 1px solid #e0e0e0; flex-shrink: 0; }
.logo { font-size: 14px; font-weight: 500; letter-spacing: 0.08em; }
.header-actions { display: flex; gap: 8px; }
.action-btn { background: #f0f0f0; border: 1px solid #ccc; border-radius: 5px; color: #333; font-size: 12px; padding: 4px 10px; cursor: pointer; }
.action-btn:hover { background: #e0e0e0; }
.overview { display: flex; align-items: center; gap: 20px; padding: 16px 20px; border-bottom: 1px solid #e0e0e0; flex-shrink: 0; }
.stats-grid { display: flex; flex-wrap: wrap; gap: 16px; align-items: center; flex: 1; }
.stat { text-align: center; }
.stat-value { font-size: 20px; font-weight: 200; }
.stat-value.accent { color: #0077cc; }
.stat-label { font-size: 10px; color: #888; text-transform: uppercase; margin-top: 2px; }
.reconcile { font-size: 11px; color: #888; }
.reconcile.warn { color: #e67e00; }
.tabs { display: flex; gap: 0; padding: 0 20px; border-bottom: 1px solid #e0e0e0; flex-shrink: 0; }
.tab-btn { background: none; border: none; border-bottom: 2px solid transparent; padding: 8px 16px; font-size: 13px; color: #888; cursor: pointer; margin-bottom: -1px; }
.tab-btn:hover { color: #333; }
.tab-btn.active { color: #0077cc; border-bottom-color: #0077cc; font-weight: 500; }
.tab-content { flex: 1; overflow-y: auto; padding: 16px 20px; }
.estimate-footer { padding: 6px 20px; font-size: 10px; color: #aaa; border-top: 1px solid #e0e0e0; flex-shrink: 0; }
.error-state { padding: 40px 20px; text-align: center; color: #888; }
</style>
```

- [ ] **Step 2: 提交**

```bash
git add src/views/Dashboard.vue
git commit -m "feat: 重构 Dashboard 为三 Tab 看板，集成 BudgetRing 圆环图"
```

---

## Task 10: App.vue 添加页面路由

**Files:**
- Modify: `src/App.vue`

用 `ref` 驱动 Dashboard ↔ Settings 页面切换，不引入 vue-router。

- [ ] **Step 1: 更新 App.vue**

```vue
<!-- src/App.vue -->
<script setup lang="ts">
import { ref } from 'vue'
import Dashboard from './views/Dashboard.vue'
import Settings from './views/Settings.vue'

const page = ref<'dashboard' | 'settings'>('dashboard')
</script>

<template>
  <Dashboard v-if="page === 'dashboard'" @openSettings="page = 'settings'" />
  <Settings v-else @back="page = 'dashboard'" />
</template>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
html, body { height: 100%; overflow: hidden; }
</style>
```

- [ ] **Step 2: 编译前端验证**

```bash
pnpm build 2>&1 | tail -30
```

预期：构建成功，无 TypeScript 错误

- [ ] **Step 3: 提交**

```bash
git add src/App.vue
git commit -m "feat: App.vue 添加 Dashboard↔Settings 页面切换"
```

---

## Task 11: 端到端集成验证

- [ ] **Step 1: 运行全量 Rust 测试**

```bash
cd src-tauri && cargo test 2>&1
```

预期：所有测试 PASS

- [ ] **Step 2: 启动开发模式验证**

```bash
pnpm tauri dev
```

手动验证清单：
- [ ] 主窗口打开，概览区显示 BudgetRing 圆环 + 关键数字
- [ ] 圆环进度随 token 消耗动态变化（蓝色为正常，橙色为超 90%）
- [ ] "项目" Tab：按 token 降序展示项目排行榜，含进度条
- [ ] "模型" Tab：饼图显示各模型消耗占比
- [ ] "时间" Tab：近 30 天柱状图（有数据时展示，无数据显示提示）
- [ ] 点击"设置"进入设置页，修改预算上限后返回，圆环进度更新
- [ ] 每 30s 前端自动刷新（可通过 Codex 运行新任务后观察）
- [ ] 对账差异率 > 5% 时显示橙色警告
- [ ] 托盘菜单和窗口切换功能正常（V0.1 功能无回归）

- [ ] **Step 3: 最终提交**

```bash
git add .
git commit -m "chore: V0.5 集成验证完成"
```

---

## 自审结果

1. **Spec 覆盖**：所有 V0.5 需求均有对应 Task：ECharts 依赖（Task 1）、聚合 command（Task 2）、定时轮询（Task 3）、预算配置（Task 4）、前端 composable（Task 5）、圆环图（Task 6）、三维度组件（Task 7）、设置页（Task 8）、Dashboard 重构（Task 9）、页面路由（Task 10）、集成验证（Task 11）。

2. **依赖顺序**：Task 1（依赖）→ Task 2-4（后端）→ Task 5（前端 composable）→ Task 6-8（前端组件）→ Task 9-10（组装）→ Task 11（验证）。

3. **V0.1 兼容性**：所有新增 command 不影响已有 get_summary/get_threads/refresh；前端 App.vue/Dashboard.vue 重构保留原有 TrayPanel 和系统托盘逻辑不变。
