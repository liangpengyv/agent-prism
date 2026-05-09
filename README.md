# AgentPrism

本地 AI Agent Token 消耗监控工具，读取 [Codex](https://github.com/openai/codex) 的本地数据，统计 Token 消耗与估算费用。

## 功能

- **总览**：Token 总量、会话数、估算费用、最活跃项目
- **项目维度**：各项目的 Token 消耗与估算费用排行
- **模型维度**：各模型的 Token 消耗与估算费用分布
- **时间维度**：近 30 天每日 Token 消耗柱状图
- **预算管理**：设定月度 Token 预算上限，圆环图直观展示进度
- **自定义价格表**：支持编辑各模型单价，一键恢复预设
- **检查更新**：设置页内手动检查是否有新版本

## 安装

需要先安装 [Homebrew](https://brew.sh)，然后执行：

```bash
brew tap liangpengyv/tap
brew install --cask agent-prism
```

升级到新版本：

```bash
brew upgrade --cask agent-prism
```

## 数据来源

AgentPrism 读取 `~/.codex/sessions/**/*.jsonl` 和 SQLite 数据库中的本地记录，所有数据处理均在本机完成，不上传任何数据。

## 开发

```bash
pnpm install
pnpm tauri dev
```

## License

MIT
