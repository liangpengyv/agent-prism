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
  tooltip: {
    trigger: 'item',
    formatter: (p: any) => `${p.name}: ${formatTokens(p.value)} tokens`,
  },
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
