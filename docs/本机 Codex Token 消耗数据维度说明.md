# 本机 Codex Token 消耗数据维度说明

生成时间：2026-05-09  
关联报告：[`本机 Codex Token 消耗统计报告.md`](./本机%20Codex%20Token%20消耗统计报告.md)  
关联方法：[`本机 Codex 消耗统计分析方法.md`](./本机%20Codex%20消耗统计分析方法.md)

## 1. 问题背景

本说明整理自一次追问：

> 区分模型的消耗统计是从哪里采集的数据，数据维度是怎样的。能够根据项目或线程维度下再区分使用了何种模型来统计消耗量和按具体模型区分估算费用么？

结论是：可以。  
Codex 本机数据中已经同时具备“线程/项目/模型/总 token”维度和“input/cached input/output/reasoning output”明细维度，只是它们来自两个不同数据源，需要关联使用。

## 2. 模型消耗统计的数据来源

模型维度主要来自本机 SQLite：

```text
~/.codex/state_5.sqlite
```

核心表：

```text
threads
```

关键字段：

| 字段 | 含义 | 用途 |
|---|---|---|
| `id` | 线程或 session id | 关联 SQLite 与 JSONL |
| `cwd` | 工作目录 | 项目维度统计 |
| `model` | 使用模型 | 例如 `gpt-5.5`、`gpt-5.4` |
| `model_provider` | 模型 provider | 例如 `seaart`、`openai` |
| `reasoning_effort` | 推理强度 | 例如 `medium`、`low` |
| `tokens_used` | 线程级总 token | SQLite 总账口径 |
| `created_at` / `updated_at` | 创建与更新时间 | 日期维度统计 |
| `title` | 线程标题 | 高消耗线程展示 |

因此，按以下维度统计总 token，SQLite 可以直接支持：

- 按模型统计。
- 按 provider 统计。
- 按项目目录 `cwd` 统计。
- 按项目目录 + 模型统计。
- 按线程 + 模型统计。
- 按日期 + 模型统计。
- 按 reasoning effort + 模型统计。

示例 SQL：

```sql
select
  cwd,
  coalesce(model, '(unknown)') as model,
  model_provider,
  count(*) as threads,
  sum(tokens_used) as tokens
from threads
group by cwd, model, model_provider
order by tokens desc;
```

## 3. 费用估算的数据来源

费用估算不能只依赖 SQLite，因为 SQLite 的 `tokens_used` 是总量，没有 input、cached input、output 拆分。

费用估算需要读取 JSONL：

```text
~/.codex/sessions/**/*.jsonl
```

重点事件：

| JSONL 事件 | 字段 | 用途 |
|---|---|---|
| `session_meta` | `id`, `cwd`, `model_provider` | 获取 session、项目目录、provider |
| `turn_context` | `model` | 获取当前线程使用模型 |
| `event_msg.payload.type == "token_count"` | `payload.info.total_token_usage` | 获取 token 类型拆分 |

`total_token_usage` 中的关键字段：

| 字段 | 含义 |
|---|---|
| `input_tokens` | 输入 token |
| `cached_input_tokens` | 命中缓存的输入 token |
| `output_tokens` | 输出 token |
| `reasoning_output_tokens` | reasoning 输出 token |
| `total_tokens` | session 累计总 token |

处理策略：

- 每个 session 文件可能有多条 `token_count`。
- `token_count` 是累计值，不能全部相加。
- 每个 session 只取最后一条有效 `token_count.total_token_usage`。
- 使用 `session_meta.id` 或文件中的 session id 与 SQLite `threads.id` 关联。

## 4. 维度关系

可以把 SQLite 和 JSONL 看作两类口径：

| 数据源 | 擅长回答 | 主要限制 |
|---|---|---|
| SQLite `threads` | 总账、线程数、项目、模型、日期、标题、provider | 缺少 input/cached/output 拆分 |
| JSONL `token_count` | token 类型拆分、费用估算 | 需要取最后一条累计值，且部分 0 token session 可能没有明细 |

推荐统计方式：

| 统计目标 | 推荐数据源 | 说明 |
|---|---|---|
| 总 token | SQLite | 作为总账 |
| 模型总 token | SQLite | `group by model, model_provider` |
| 项目 + 模型总 token | SQLite | `group by cwd, model, model_provider` |
| 线程 + 模型总 token | SQLite | 每条 thread 直接有 model 和 tokens_used |
| input/cached/output 拆分 | JSONL | 每个 session 取最后一条有效 token_count |
| 模型费用估算 | JSONL | 按模型价格分别计算 |
| 项目 + 模型费用估算 | JSONL + SQLite | JSONL 取拆分，按 cwd/model 汇总 |
| 线程 + 模型费用估算 | JSONL + SQLite | JSONL 算费用，SQLite 补标题和时间 |

## 5. 费用估算公式

费用按模型分别计算：

```text
uncached_input_tokens = input_tokens - cached_input_tokens

费用 =
  uncached_input_tokens / 1,000,000 * 输入单价
+ cached_input_tokens / 1,000,000 * 缓存输入单价
+ output_tokens / 1,000,000 * 输出单价
```

`reasoning_output_tokens` 在当前统计中作为明细展示，不重复计入费用公式，因为本地记录中的：

```text
total_tokens = input_tokens + output_tokens
```

## 6. 项目 + 模型维度示例

以下为当时补跑得到的项目 + 模型 + provider + 费用估算示例：

| 项目 | 模型 | provider | session | total token | 估算费用 |
|---|---|---|---:|---:|---:|
| `/Users/mac038/repos/website-ssr` | `gpt-5.5` | `seaart` | 26 | 66,337,565 | `$65.18` |
| `/Users/mac038/repos/website-ssr-h5` | `gpt-5.5` | `seaart` | 16 | 28,717,926 | `$30.76` |
| `/Users/mac038/repos/discord/discord_pubopinion` | `gpt-5.5` | `seaart` | 1 | 3,105,082 | `$4.68` |
| `/Users/mac038/repos/website-packages` | `gpt-5.5` | `seaart` | 8 | 2,557,791 | `$4.64` |
| `/Users/mac038/repos/agent-prism` | `gpt-5.5` | `seaart` | 2 | 963,669 | `$1.45` |
| `/Users/mac038/Documents/Codex/2026-04-22-hello-2` | `gpt-5.4` | `seaart` | 1 | 55,679 | `$0.08` |

说明：这些数字是当时查询本机日志得到的快照。由于当前会话继续运行，`agent-prism` 等当前项目的数值可能会继续增长。

## 7. 线程 + 模型维度

线程维度也可以按具体模型计算消耗和费用。推荐输出字段：

| 字段 | 来源 | 说明 |
|---|---|---|
| `thread_id` | SQLite / JSONL | 线程 ID |
| `created_at` | SQLite | 创建时间 |
| `cwd` | SQLite / JSONL | 项目目录 |
| `title` | SQLite | 线程标题 |
| `model` | SQLite / JSONL | 使用模型 |
| `model_provider` | SQLite / JSONL | provider |
| `input_tokens` | JSONL | 输入 token |
| `cached_input_tokens` | JSONL | 缓存输入 token |
| `output_tokens` | JSONL | 输出 token |
| `reasoning_output_tokens` | JSONL | reasoning 输出 token |
| `total_tokens` | JSONL / SQLite | 总 token |
| `estimated_cost_usd` | 计算结果 | 估算费用 |

示例输出形态：

```text
thread_id | created_at | cwd | model | provider | total | input | cached | output | reasoning | estimated_cost_usd | title
```

## 8. 重要边界

当前统计默认采用“一个线程归属一个模型”的口径。这适用于本机大多数 Codex session，因为线程创建时的 `turn_context.model` 与 SQLite `threads.model` 通常一致。

如果未来出现同一个 session 中途切换模型，则需要更细的算法：

1. 按时间顺序读取同一 JSONL 文件中的所有 `turn_context` 与有效 `token_count`。
2. 对相邻 `token_count.total_token_usage` 做 delta，得到每一段新增 token。
3. 将这段新增 token 归因到当时最新的 `turn_context.model`。
4. 再按模型分别聚合和估算费用。

如果不做 delta 拆分，而只使用最后一条累计 `token_count`，那么同一 session 内的多模型切换会被整体归因到最后或主线程模型，费用仍可估算，但模型归因不够精确。

## 9. 结论

可以根据项目或线程维度，再区分使用了何种模型来统计消耗量，并按具体模型估算费用。

推荐实践：

- SQLite 负责总账和维度归属。
- JSONL 负责 token 类型拆分和费用估算。
- 通过 session/thread id 将二者关联。
- 默认按线程归属模型统计。
- 如需支持 session 内多模型切换，再引入 token_count delta 归因算法。
