<!-- src/components/DayChart.vue -->
<script setup lang="ts">
import { computed } from 'vue'
import { use } from 'echarts/core'
import { BarChart } from 'echarts/charts'
import { GridComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import VChart from 'vue-echarts'
import type { DayStat } from '../composables/useAggregates'

use([BarChart, GridComponent, TooltipComponent, CanvasRenderer])

const props = defineProps<{ stats: DayStat[] }>()

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return String(n)
}

const option = computed(() => ({
  tooltip: {
    trigger: 'axis',
    axisPointer: { type: 'shadow' },
  },
  grid: { left: 48, right: 24, top: 16, bottom: 44 },
  xAxis: {
    type: 'category',
    data: props.stats.map(d => d.date.slice(5)),
    axisLabel: { fontSize: 10, color: '#aaa', rotate: 45 },
    axisLine: { lineStyle: { color: '#e0e0e0' } },
  },
  yAxis: {
    type: 'value',
    name: 'Token',
    axisLabel: { formatter: (v: number) => formatTokens(v), fontSize: 10, color: '#aaa' },
    splitLine: { lineStyle: { color: '#f5f5f5' } },
  },
  series: [
    {
      name: 'Token 消耗',
      type: 'bar',
      data: props.stats.map(d => d.tokens),
      itemStyle: { color: '#4FC3F7', borderRadius: [2, 2, 0, 0] },
      barMaxWidth: 24,
    },
  ],
}))
</script>

<template>
  <div class="day-chart">
    <div v-if="stats.length === 0" class="empty">近 30 天暂无数据</div>
    <VChart v-else :option="option" autoresize style="width:100%;height:240px;" />
  </div>
</template>

<style scoped>
.day-chart { width: 100%; }
.empty { color: #aaa; font-size: 13px; text-align: center; padding: 40px; }
</style>
