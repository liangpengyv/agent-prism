# 本机 Codex Token 消耗统计报告

生成时间：2026-05-08  
统计对象：`~/.codex/` 本机落盘数据  
参考方法：[`本机 Codex 消耗统计分析方法.md`](./本机%20Codex%20消耗统计分析方法.md)

## 1. 统计口径

本报告按以下口径统计：

- SQLite 总账：读取 `~/.codex/state_5.sqlite` 中 `threads.tokens_used`。
- JSONL 明细：扫描 `~/.codex/sessions/**/*.jsonl`，每个 session 只取最后一条有效 `token_count.total_token_usage`。
- 费用估算：按模型区分 input、cached input、output，使用公开 API 标准价估算。

注意：本地数据可以确认已落盘的会话和 token 统计，但不能代表真实账单、套餐抵扣、内部 provider 折扣或服务端最终扣费。

## 2. 总览

| 口径 | 会话数 | 总 token |
|---|---:|---:|
| SQLite `threads.tokens_used` | 62 | 101,939,337 |
| JSONL 最后一条 `token_count` | 56 有效 / 62 文件 | 101,939,337 |
| 差异 | 0 | 0.00% |

JSONL 中有 6 个 session 没有有效 token 明细，但这些 session 在 SQLite 总账中均为 `0 token`，因此两套口径完全对齐。

## 3. 按模型拆分

| 模型 | provider | SQLite 线程数 | SQLite token |
|---|---|---:|---:|
| `gpt-5.5` | `seaart` | 59 | 101,883,658 |
| `gpt-5.4` | `seaart` | 1 | 55,679 |
| `codex-auto-review` | `seaart` | 1 | 0 |
| `gpt-5.4` | `openai` | 1 | 0 |
| **合计** |  | **62** | **101,939,337** |

## 4. Token 类型拆分

| 模型 | 有效 session | input | cached input | uncached input | output | reasoning output | total |
|---|---:|---:|---:|---:|---:|---:|---:|
| `gpt-5.5` | 55 | 101,393,333 | 92,132,992 | 9,260,341 | 490,325 | 102,794 | 101,883,658 |
| `gpt-5.4` | 1 | 55,383 | 27,136 | 28,247 | 296 | 110 | 55,679 |
| **合计** | **56** | **101,448,716** | **92,160,128** | **9,288,588** | **490,621** | **102,904** | **101,939,337** |

说明：

- `uncached input = input - cached input`。
- 本地记录中 `total_tokens = input_tokens + output_tokens`。
- `reasoning_output_tokens` 在这里作为明细展示，不在费用公式中重复计入。

## 5. 费用估算

价格来源：OpenAI API Pricing，https://openai.com/api/pricing/

采用价格：

| 模型 | input / 1M | cached input / 1M | output / 1M |
|---|---:|---:|---:|
| `gpt-5.5` | `$5.00` | `$0.50` | `$30.00` |
| `gpt-5.4` | `$2.50` | `$0.25` | `$15.00` |

估算公式：

```text
费用 =
  uncached_input_tokens / 1,000,000 * 输入单价
+ cached_input_tokens / 1,000,000 * 缓存输入单价
+ output_tokens / 1,000,000 * 输出单价
```

估算结果：

| 模型 | uncached input | cached input | output | 估算费用 |
|---|---:|---:|---:|---:|
| `gpt-5.5` | 9,260,341 | 92,132,992 | 490,325 | `$107.08` |
| `gpt-5.4` | 28,247 | 27,136 | 296 | `$0.08` |
| **合计** | **9,288,588** | **92,160,128** | **490,621** | **`$107.16`** |

结论：本机已落盘 Codex 使用量约 **1.019 亿 token**，按公开 API 标准价粗估约 **`$107.16`**。

该结果是估算费用，不等同于真实账单。当前本机 provider 主要显示为 `seaart`，如果存在内部折扣、套餐抵扣、不同计费规则或未落盘记录，实际费用会不同。

## 6. 项目目录分布

| cwd | 线程数 | token |
|---|---:|---:|
| `/Users/mac038/repos/website-ssr` | 27 | 66,337,565 |
| `/Users/mac038/repos/website-ssr-h5` | 17 | 28,717,926 |
| `/Users/mac038/repos/discord/discord_pubopinion` | 1 | 3,105,082 |
| `/Users/mac038/repos/website-packages` | 9 | 2,557,791 |
| `/Users/mac038` | 2 | 585,618 |
| `/Users/mac038/repos/agent-prism` | 2 | 340,364 |
| `/Users/mac038/repos/ask-codex` | 1 | 239,312 |
| `/Users/mac038/Documents/Codex/2026-04-22-hello-2` | 1 | 55,679 |

## 7. 日期分布 Top 8

| 日期 | 线程数 | token |
|---|---:|---:|
| 2026-04-27 | 6 | 27,675,807 |
| 2026-04-28 | 7 | 20,490,557 |
| 2026-04-25 | 6 | 14,008,904 |
| 2026-04-30 | 8 | 12,827,670 |
| 2026-05-08 | 7 | 6,774,562 |
| 2026-05-06 | 6 | 6,702,254 |
| 2026-04-29 | 4 | 6,582,671 |
| 2026-05-07 | 6 | 3,551,008 |

## 8. 高消耗线程 Top 10

| 线程 ID | 创建时间 | 模型 | token | cwd | 标题 |
|---|---|---|---:|---|---|
| `019dd1d8-697c-7302-902e-ec8a01d9fef2` | 2026-04-28 10:08:34 | `gpt-5.5` | 15,655,787 | `/Users/mac038/repos/website-ssr` | 新增 postDetail 隐藏信息标记 |
| `019dcd9e-1d5a-7110-ab30-7ef65a15fbb0` | 2026-04-27 14:26:24 | `gpt-5.5` | 11,726,903 | `/Users/mac038/repos/website-ssr-h5` | Refactor view module guideline |
| `019dcda5-2398-73e0-9387-08b808d92d58` | 2026-04-27 14:34:05 | `gpt-5.5` | 6,579,809 | `/Users/mac038/repos/website-ssr` | 定位 circle-right 滚动容器 |
| `019dc2da-2774-7101-88ad-9f56ff68fcca` | 2026-04-25 12:16:10 | `gpt-5.5` | 5,934,674 | `/Users/mac038/repos/website-ssr` | 排查后退滚动位置失效 |
| `019dfb2f-d3ff-77b2-bab5-b8e913d805aa` | 2026-05-06 10:48:28 | `gpt-5.5` | 5,245,775 | `/Users/mac038/repos/website-ssr` | 分析 int# 标识区别 |
| `019dccbf-daf9-7d40-8eb0-efc747cf75c7` | 2026-04-27 10:23:38 | `gpt-5.5` | 4,811,824 | `/Users/mac038/repos/website-ssr` | 分析组件分组方案 |
| `019ddc8b-720a-7b62-b117-741221950b4c` | 2026-04-30 12:00:19 | `gpt-5.5` | 4,652,313 | `/Users/mac038/repos/website-ssr-h5` | 排查 articleDetail 跳转延迟 |
| `019dcf1b-976c-73a1-8b36-df53b98a88d2` | 2026-04-27 21:23:05 | `gpt-5.5` | 4,295,666 | `/Users/mac038/repos/website-ssr` | 抽取 CircleLeft 组件 |
| `019dc3c7-d76b-7ab1-9c3e-2f06735a6148` | 2026-04-25 16:35:47 | `gpt-5.5` | 3,630,401 | `/Users/mac038/repos/website-ssr` | 分析页面保活滚动恢复 |
| `019ddd1a-6c25-7bc3-92b7-6f671c0f1d2b` | 2026-04-30 14:36:29 | `gpt-5.5` | 3,165,261 | `/Users/mac038/repos/website-ssr` | 修复下拉项展开状态区分 |

## 9. 可信边界

本地可确认：

- 本机已保存的 Codex 线程数和 session 文件数。
- 本机已保存的 token 总量。
- 按模型、provider、项目目录、日期的分布。
- JSONL 中可见的 input、cached input、output、reasoning output 拆分。

只能估算：

- 实际费用和最终账单。
- 团队账号、套餐、折扣、内部 provider 计费规则。
- 被清理、迁移、未落盘或服务端才有的历史数据。
