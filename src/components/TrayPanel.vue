<!-- src/components/TrayPanel.vue -->
<script setup lang="ts">
import { onMounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useStats } from '../composables/useStats'
import { useAgentSwitch } from '../composables/useAgentSwitch'

const { summary, warnings, error, loading, loadSummary } = useStats()
const { currentAgent, init: initAgent } = useAgentSwitch()

onMounted(async () => {
  await initAgent()
  await loadSummary(currentAgent.value)
})

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return String(n)
}

function formatCost(usd: number): string {
  return '$' + usd.toFixed(4)
}

async function openDashboard() {
  const win = getCurrentWindow()
  await win.show()
  await win.setFocus()
}
</script>

<template>
  <div class="tray-panel">
    <div v-if="loading" class="state-empty">加载中…</div>
    <div v-else-if="error" class="state-empty">{{ error }}</div>
    <div v-else-if="!summary" class="state-empty">未检测到 Codex 数据</div>
    <template v-else>
      <div class="metric-row">
        <span class="metric-label">Token 总量</span>
        <span class="metric-value">{{ formatTokens(summary.total_tokens) }}</span>
      </div>
      <div class="metric-row">
        <span class="metric-label">估算费用</span>
        <span class="metric-value accent">{{ formatCost(summary.estimated_cost_usd) }}</span>
      </div>
      <div class="metric-row" v-if="summary.top_project">
        <span class="metric-label">最活跃项目</span>
        <span class="metric-value project">{{ summary.top_project.split('/').at(-1) }}</span>
      </div>
      <div class="estimate-note">估算，非真实账单</div>
      <div v-if="warnings.length > 0" class="warning-note">⚠ 部分数据可能不完整</div>
    </template>
    <button class="open-btn" @click="openDashboard">打开看板</button>
  </div>
</template>

<style scoped>
.tray-panel {
  padding: 16px;
  min-width: 220px;
  font-family: -apple-system, sans-serif;
  color: #f0f0f0;
}
.metric-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 8px;
}
.metric-label {
  font-size: 11px;
  color: #888;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.metric-value {
  font-size: 20px;
  font-weight: 200;
  color: #f0f0f0;
}
.metric-value.accent { color: #4FC3F7; }
.metric-value.project { font-size: 13px; font-weight: 400; color: #ccc; }
.estimate-note {
  font-size: 10px;
  color: #555;
  margin-top: 4px;
}
.warning-note {
  font-size: 11px;
  color: #FFB74D;
  margin-top: 6px;
}
.open-btn {
  margin-top: 12px;
  width: 100%;
  padding: 6px;
  background: rgba(255,255,255,0.08);
  border: 1px solid rgba(255,255,255,0.12);
  border-radius: 6px;
  color: #f0f0f0;
  font-size: 12px;
  cursor: pointer;
}
.open-btn:hover { background: rgba(255,255,255,0.14); }
.state-empty { font-size: 13px; color: #666; padding: 8px 0; }
</style>
