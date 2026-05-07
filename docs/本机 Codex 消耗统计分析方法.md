# 本机 Codex 消耗统计分析方法
## 1. 背景
Codex 在本机运行时，会在用户目录下保存会话、线程、日志和状态数据。这些本地数据可以用于还原一段时间内的使用情况，包括会话数量、token 总量、输入/输出拆分、模型分布、项目目录分布和高消耗任务等。

本文档沉淀一套通用的“统计本机 Codex 消耗”分析方法，不绑定具体业务项目。后续可以在此基础上继续扩展为本地统计小工具的产品开发文档。

## 2. 核心目标
统计工具应回答以下问题：

+ 本机 Codex 总共使用了多少 token？
+ 有多少次会话或线程？
+ 消耗主要集中在哪些项目目录？
+ 使用了哪些模型和 provider？
+ 输入、缓存输入、输出、reasoning 输出分别是多少？
+ 哪些会话最耗 token？
+ 哪些日期消耗最高？
+ 按公开价目估算大约费用是多少？
+ 哪些数据是本地可确认的，哪些只是估算？

## 3. 数据来源
优先读取 Codex 本地目录：

```bash
~/.codex/
```

主要数据源：

```bash
~/.codex/state_*.sqlite
~/.codex/sessions/**/*.jsonl
~/.codex/session_index.jsonl
```

推荐优先级：

1. `state_*.sqlite`：用于线程级汇总，通常适合做总账。
2. `sessions/**/*.jsonl`：用于 token 类型拆分，适合做输入、缓存输入、输出、reasoning 输出分析。
3. `session_index.jsonl`：用于补充线程标题、更新时间等展示信息。

## 4. SQLite 统计口径
核心表通常是：

```sql
threads
```

重点字段：

```latex
id
created_at
updated_at
cwd
title
source
model_provider
model
tokens_used
```

适合统计：

+ 总线程数
+ 总 token
+ 日期分布
+ 项目目录分布
+ 模型分布
+ provider 分布
+ 高消耗线程排行

典型 SQL：

```sql
select count(*), sum(tokens_used)
from threads;

select cwd, count(*), sum(tokens_used)
from threads
group by cwd
order by sum(tokens_used) desc;

select model, model_provider, count(*), sum(tokens_used)
from threads
group by model, model_provider
order by sum(tokens_used) desc;
```

## 5. JSONL 统计口径
`sessions/**/*.jsonl` 文件中重点关注两类事件：

```latex
session_meta
event_msg.payload.type == "token_count"
```

`session_meta` 通常提供：

```latex
id
timestamp
cwd
model_provider
cli_version
source
```

`token_count` 通常提供：

```latex
input_tokens
cached_input_tokens
output_tokens
reasoning_output_tokens
total_tokens
```

处理策略：

+ 每个 session 文件可能有多条 `token_count`。
+ 不应把所有 `token_count` 直接相加。
+ 应取每个 session 最后一条有效 `token_count`。
+ 该条记录代表该 session 当前累计消耗。

这是避免重复统计的关键。

## 6. 统计维度设计
### 时间维度
```latex
按日
按周
按月
全部时间
自定义时间范围
```

### 项目维度
```latex
cwd
repo 名称
目录层级归类
未知目录
```

### 模型维度
```latex
model
model_provider
source
reasoning_effort
```

### Token 类型维度
```latex
input_tokens
cached_input_tokens
uncached_input_tokens
output_tokens
reasoning_output_tokens
total_tokens
```

其中：

```latex
uncached_input_tokens = input_tokens - cached_input_tokens
```

### 会话维度
```latex
线程 ID
标题
创建时间
更新时间
工作目录
模型
总 token
```

### 榜单维度
```latex
最高消耗会话 Top N
最高消耗目录 Top N
最高消耗日期 Top N
最高消耗模型 Top N
```

## 7. 总量校验策略
推荐同时计算两套口径：

+ SQLite 总账：`sum(threads.tokens_used)`
+ JSONL 明细：每个 session 最后一条 `token_count.total_tokens` 求和

然后展示差异：

```latex
差异 = JSONL 总量 - SQLite 总量
差异率 = 差异 / SQLite 总量
```

解释规则：

+ 差异很小：认为两套口径基本一致。
+ SQLite 更完整：优先作为总账。
+ JSONL 更细：优先用于 token 类型拆分。
+ 差异很大：提示用户可能存在日志缺失、迁移、版本差异或未落盘记录。

## 8. 费用估算方式
本地通常不会保存真实账单或实际扣费信息，因此费用只能估算。

基础公式：

```latex
费用 =
  uncached_input_tokens / 1,000,000 * 输入单价
+ cached_input_tokens / 1,000,000 * 缓存输入单价
+ output_tokens / 1,000,000 * 输出单价
```

其中：

```latex
uncached_input_tokens = input_tokens - cached_input_tokens
```

价格表应设计为可配置，而不是写死。

示例结构：

```json
{
  "gpt-5.5": {
    "input_per_1m": 5,
    "cached_input_per_1m": 0.5,
    "output_per_1m": 30
  },
  "gpt-5.4": {
    "input_per_1m": 2.5,
    "cached_input_per_1m": 0.25,
    "output_per_1m": 15
  }
}
```

费用结果必须标注为：

```latex
估算费用，不等同于真实账单
```

## 9. 可信边界
工具应明确区分“可确认数据”和“估算数据”。

本地可确认：

+ 本机已保存的会话数量
+ 本机已保存的 token 统计
+ 项目目录分布
+ 模型/provider 分布
+ 会话标题和时间
+ token 类型拆分

只能估算：

+ 实际费用
+ 服务端真实扣费
+ 团队账号账单
+ 套餐抵扣
+ 内部 provider 折扣
+ 被清理或未落盘的历史数据

## 10. 产品化建议
后续可以把这套方法发展成一个本地 CLI 小工具。

核心命令设想：

```bash
codex-usage summary
codex-usage by-project
codex-usage by-model
codex-usage by-day
codex-usage top
codex-usage estimate-cost
codex-usage export --format json
codex-usage export --format csv
```

推荐默认输出：

```latex
总 token
估算费用
统计时间范围
会话数量
项目 Top 5
模型 Top 5
高消耗会话 Top 10
数据可信说明
```

高级能力：

```latex
自定义 Codex 目录
自定义价格表
按时间过滤
按项目过滤
导出 JSON/CSV
生成 Markdown 报告
检测 SQLite 与 JSONL 差异
提示异常会话
```

## 11. 设计原则
这个工具的关键不是“算一个数字”，而是提供清晰、可解释、可复核的本机消耗画像。

推荐原则：

+ SQLite 做总账。
+ JSONL 做明细。
+ 每个 session 只取最后一条 token 累计。
+ 费用必须显式标注为估算。
+ 价格表必须可配置。
+ 不绑定具体项目路径。
+ 不上传数据，默认只做本地只读分析。
+ 输出同时面向人读和机器处理。

## 12. 最小可行版本
MVP 可以只做五件事：

1. 自动发现 `~/.codex/state_*.sqlite`。
2. 自动扫描 `~/.codex/sessions/**/*.jsonl`。
3. 输出总 token、会话数、时间范围。
4. 输出项目、模型、日期三个维度统计。
5. 根据可配置价格表估算费用。

这份文档可以继续扩展为正式产品开发文档，下一步适合补充用户故事、命令设计、数据结构、异常处理、输出格式和开发里程碑。

