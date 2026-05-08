<!-- src/components/ProjectList.vue -->
<script setup lang="ts">
import { computed } from 'vue'
import type { ProjectStat } from '../composables/useAggregates'

const props = defineProps<{ stats: ProjectStat[] }>()

const maxTokens = computed(() =>
  props.stats.length > 0 ? props.stats[0].tokens : 1
)

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
