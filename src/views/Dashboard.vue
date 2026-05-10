<!-- src/views/Dashboard.vue -->
<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useStats, useDataUpdatedListener } from '../composables/useStats'
import { useAggregates } from '../composables/useAggregates'
import { AGENTS } from '../composables/useAgentSwitch'
import BudgetRing from '../components/BudgetRing.vue'
import ProjectList from '../components/ProjectList.vue'
import ModelBreakdown from '../components/ModelBreakdown.vue'
import DayChart from '../components/DayChart.vue'
import AgentSwitcher from '../components/AgentSwitcher.vue'
import type { CommandResult } from '../composables/useStats'
import type { AgentId } from '../composables/useAgentSwitch'

const props = defineProps<{ currentAgent: AgentId }>()
const emit = defineEmits<{ openSettings: []; agentChange: [agent: AgentId] }>()

const { summary, error, loading, loadSummary } = useStats()
const { byProject, byModel, byDate, loadAll } = useAggregates()
const activeTab = ref<'project' | 'model' | 'date'>('project')
const budgetTokens = ref(10_000_000)

async function loadBudget(agent: string) {
  const res = await invoke<CommandResult<number | null>>('get_budget', { agent })
  if (res.data != null) budgetTokens.value = res.data
}

async function reload() {
  await Promise.all([loadSummary(props.currentAgent), loadAll(props.currentAgent), loadBudget(props.currentAgent)])
}

function handleAgentChange(agent: AgentId) {
  emit('agentChange', agent)
}

onMounted(async () => {
  await reload()
})

watch(() => props.currentAgent, async () => {
  await reload()
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
      <AgentSwitcher
        :currentAgent="props.currentAgent"
        :agents="AGENTS"
        @change="handleAgentChange"
      />
      <div class="header-actions">
        <button class="action-btn" @click="reload" :disabled="loading">
          {{ loading ? '刷新中…' : '刷新' }}
        </button>
        <button class="action-btn" @click="$emit('openSettings')">设置</button>
      </div>
    </header>

    <div v-if="error" class="error-state">
      <template v-if="props.currentAgent === 'codex'">未检测到 ~/.codex 目录</template>
      <template v-else>未检测到 ~/.claude 目录</template>
    </div>

    <template v-else>
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
            <div class="stat-value">{{ summary.session_count }}</div>
            <div class="stat-label">Session 数</div>
          </div>
          <div v-if="props.currentAgent === 'codex'" class="stat">
            <div class="stat-value">{{ summary.thread_count }}</div>
            <div class="stat-label">线程数</div>
          </div>
          <div
            v-if="props.currentAgent === 'codex' && summary.reconcile.diff_rate >= 0"
            class="reconcile"
            :class="{ warn: summary.reconcile.warning }"
          >
            对账差异率 {{ (summary.reconcile.diff_rate * 100).toFixed(1) }}%
          </div>
        </div>
      </div>

      <div class="tabs">
        <button
          v-for="[key, label] in [['project','项目'],['model','模型'],['date','时间']] as [string, string][]"
          :key="key"
          class="tab-btn"
          :class="{ active: activeTab === key }"
          @click="activeTab = key as 'project' | 'model' | 'date'"
        >{{ label }}</button>
      </div>

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
.header-actions { display: flex; gap: 8px; }
.action-btn { background: #f0f0f0; border: 1px solid #ccc; border-radius: 5px; color: #333; font-size: 12px; padding: 4px 10px; cursor: pointer; }
.action-btn:hover { background: #e0e0e0; }
.action-btn:disabled { opacity: 0.5; cursor: default; }
.overview { display: flex; align-items: center; gap: 20px; padding: 12px 20px; border-bottom: 1px solid #e0e0e0; flex-shrink: 0; }
.stats-grid { display: flex; flex-wrap: wrap; gap: 16px; align-items: center; flex: 1; }
.stat { text-align: center; }
.stat-value { font-size: 20px; font-weight: 200; }
.stat-value.accent { color: #0077cc; }
.stat-label { font-size: 10px; color: #888; text-transform: uppercase; margin-top: 2px; }
.reconcile { font-size: 11px; color: #888; }
.reconcile.warn { color: #e67e00; }
.tabs { display: flex; padding: 0 20px; border-bottom: 1px solid #e0e0e0; flex-shrink: 0; }
.tab-btn { background: none; border: none; border-bottom: 2px solid transparent; padding: 8px 16px; font-size: 13px; color: #888; cursor: pointer; margin-bottom: -1px; }
.tab-btn:hover { color: #333; }
.tab-btn.active { color: #0077cc; border-bottom-color: #0077cc; font-weight: 500; }
.tab-content { flex: 1; overflow-y: auto; padding: 16px 20px; }
.estimate-footer { padding: 6px 20px; font-size: 10px; color: #aaa; border-top: 1px solid #e0e0e0; flex-shrink: 0; }
.error-state { padding: 40px 20px; text-align: center; color: #888; }
</style>
