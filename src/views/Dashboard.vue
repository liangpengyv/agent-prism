<!-- src/views/Dashboard.vue -->
<script setup lang="ts">
import { onMounted } from 'vue'
import { useStats } from '../composables/useStats'
import ThreadList from '../components/ThreadList.vue'

const { summary, threads, warnings, error, loading, loadSummary, loadThreads, refresh } = useStats()

onMounted(async () => {
  await loadSummary()
  await loadThreads()
})

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
      <button class="refresh-btn" @click="refresh" :disabled="loading">
        {{ loading ? '刷新中…' : '刷新' }}
      </button>
    </header>

    <div v-if="error" class="error-state">{{ error }}</div>

    <template v-else>
      <div class="summary-bar" v-if="summary">
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

      <div v-if="warnings.length > 0" class="warnings">
        <span v-for="w in warnings" :key="w" class="warning-item">⚠ {{ w }}</span>
      </div>

      <div class="section-title">线程列表</div>
      <div class="list-container">
        <ThreadList :threads="threads" />
      </div>

      <div class="estimate-footer">估算，非真实账单</div>
    </template>
  </div>
</template>

<style scoped>
.dashboard {
  display: flex;
  flex-direction: column;
  height: 100vh;
  font-family: -apple-system, sans-serif;
  color: #333;
  overflow: hidden;
}
.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 20px;
  border-bottom: 1px solid #e0e0e0;
}
.logo { font-size: 14px; font-weight: 500; letter-spacing: 0.08em; }
.refresh-btn {
  background: #f0f0f0;
  border: 1px solid #ccc;
  border-radius: 5px;
  color: #333;
  font-size: 12px;
  padding: 4px 10px;
  cursor: pointer;
}
.refresh-btn:hover { background: #e0e0e0; }
.summary-bar {
  display: flex;
  align-items: center;
  gap: 24px;
  padding: 16px 20px;
  border-bottom: 1px solid #e0e0e0;
}
.stat { text-align: center; }
.stat-value { font-size: 22px; font-weight: 200; }
.stat-value.accent { color: #0077cc; }
.stat-label { font-size: 10px; color: #888; text-transform: uppercase; margin-top: 2px; }
.reconcile { margin-left: auto; font-size: 11px; color: #888; }
.reconcile.warn { color: #e67e00; }
.warnings { padding: 8px 20px; display: flex; flex-direction: column; gap: 2px; }
.warning-item { font-size: 11px; color: #e67e00; }
.section-title { padding: 10px 20px 4px; font-size: 11px; color: #888; text-transform: uppercase; letter-spacing: 0.06em; }
.list-container { flex: 1; overflow: hidden; }
.estimate-footer { padding: 8px 20px; font-size: 10px; color: #aaa; border-top: 1px solid #e0e0e0; }
.error-state { padding: 40px 20px; text-align: center; color: #888; }
</style>


<template>
  <div class="dashboard">
    <header class="header">
      <button class="close-btn" @mousedown.stop @click.stop="closeWindow" title="关闭">
        <span class="close-dot"></span>
      </button>
      <span class="logo">AgentPrism</span>
      <button class="refresh-btn" @click="refresh" :disabled="loading">
        {{ loading ? '刷新中…' : '刷新' }}
      </button>
    </header>

    <div v-if="error" class="error-state">{{ error }}</div>

    <template v-else>
      <div class="summary-bar" v-if="summary">
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

      <div v-if="warnings.length > 0" class="warnings">
        <span v-for="w in warnings" :key="w" class="warning-item">⚠ {{ w }}</span>
      </div>

      <div class="section-title">线程列表</div>
      <div class="list-container">
        <ThreadList :threads="threads" />
      </div>

      <div class="estimate-footer">估算，非真实账单</div>
    </template>
  </div>
</template>

<style scoped>
.dashboard {
  display: flex;
  flex-direction: column;
  height: 100vh;
  font-family: -apple-system, sans-serif;
  color: #e0e0e0;
  overflow: hidden;
}
.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px 20px;
  border-bottom: 1px solid rgba(255,255,255,0.06);
  -webkit-app-region: drag;
}
.close-btn {
  -webkit-app-region: no-drag;
  pointer-events: all;
  width: 13px;
  height: 13px;
  border-radius: 50%;
  background: #ff5f57;
  border: none;
  cursor: pointer;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  margin-right: 8px;
}
.close-btn:hover { background: #e0443e; }
.close-dot { display: none; }
.logo { font-size: 14px; font-weight: 500; letter-spacing: 0.08em; color: #fff; }
.refresh-btn {
  -webkit-app-region: no-drag;
  background: rgba(255,255,255,0.07);
  border: 1px solid rgba(255,255,255,0.1);
  border-radius: 5px;
  color: #ccc;
  font-size: 12px;
  padding: 4px 10px;
  cursor: pointer;
}
.refresh-btn:hover { background: rgba(255,255,255,0.12); }
.summary-bar {
  display: flex;
  align-items: center;
  gap: 24px;
  padding: 16px 20px;
  border-bottom: 1px solid rgba(255,255,255,0.06);
}
.stat { text-align: center; }
.stat-value { font-size: 22px; font-weight: 200; }
.stat-value.accent { color: #4FC3F7; }
.stat-label { font-size: 10px; color: #555; text-transform: uppercase; margin-top: 2px; }
.reconcile { margin-left: auto; font-size: 11px; color: #555; }
.reconcile.warn { color: #FFB74D; }
.warnings { padding: 8px 20px; display: flex; flex-direction: column; gap: 2px; }
.warning-item { font-size: 11px; color: #FFB74D; }
.section-title { padding: 10px 20px 4px; font-size: 11px; color: #444; text-transform: uppercase; letter-spacing: 0.06em; }
.list-container { flex: 1; overflow: hidden; }
.estimate-footer { padding: 8px 20px; font-size: 10px; color: #444; border-top: 1px solid rgba(255,255,255,0.04); }
.error-state { padding: 40px 20px; text-align: center; color: #666; }
</style>
