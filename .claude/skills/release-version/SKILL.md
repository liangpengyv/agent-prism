---
name: release-version
description: Use when releasing a new version of AgentPrism - bumping version numbers, tagging, and triggering CI build
---

# Release Version

## Overview

AgentPrism 使用 git tag 触发 GitHub Actions 自动构建 macOS 安装包并发布到 Releases。版本号格式为 `x.y.z-alpha`。

## 版本号规则

- 格式：`x.y.z-alpha`（当前阶段所有版本携带 `-alpha` 后缀）
- 示例：`0.1.0-alpha` → `0.1.1-alpha` → `0.2.0-alpha`
- 版本号存在于三处，必须同步更新：

| 文件 | 字段 |
|------|------|
| `src-tauri/tauri.conf.json` | `"version"` |
| `src-tauri/Cargo.toml` | `version`（package 段第一行） |
| `package.json` | `"version"` |

## 发版流程

**1. 更新三处版本号**

```bash
# 将 OLD 替换为当前版本，NEW 替换为新版本
sed -i '' 's/"version": "OLD"/"version": "NEW"/' src-tauri/tauri.conf.json package.json
sed -i '' 's/^version = "OLD"/version = "NEW"/' src-tauri/Cargo.toml
```

**2. 验证编译**

```bash
cd src-tauri && cargo check
```

**3. 提交并推送**

```bash
git add src-tauri/tauri.conf.json src-tauri/Cargo.toml package.json
git commit -m "chore: bump version to NEW"
git push
```

**4. 打 tag 并推送（触发 CI 构建）**

```bash
git tag vNEW
git push origin vNEW
```

推送 tag 后 GitHub Actions 自动启动，约 10-15 分钟完成构建，产物（`.dmg`）自动上传到 GitHub Releases 页面。含 `alpha` 的 tag 会标记为 Pre-release。

## 注意事项

- tag 名称必须以 `v` 开头，例如 `v0.1.1-alpha`
- 不要在本地手动构建后上传，统一由 CI 产出产物
- 推送 tag 前务必确认代码已推送到 main，否则 CI 拉取的是旧代码
