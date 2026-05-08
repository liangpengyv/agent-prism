<!-- src/components/BudgetRing.vue -->
<script setup lang="ts">
import { computed } from 'vue'
import { use } from 'echarts/core'
import { GaugeChart } from 'echarts/charts'
import { CanvasRenderer } from 'echarts/renderers'
import VChart from 'vue-echarts'

use([GaugeChart, CanvasRenderer])

const props = defineProps<{
  usedTokens: number
  budgetTokens: number
}>()

const percent = computed(() =>
  props.budgetTokens > 0
    ? Math.min(100, Math.round((props.usedTokens / props.budgetTokens) * 100))
    : 0
)

const color = computed(() =>
  percent.value >= 90 ? '#FFB74D' : '#4FC3F7'
)

const option = computed(() => ({
  series: [{
    type: 'gauge',
    startAngle: 210,
    endAngle: -30,
    min: 0,
    max: 100,
    splitNumber: 0,
    radius: '88%',
    axisLine: {
      lineStyle: {
        width: 10,
        color: [[percent.value / 100, color.value], [1, 'rgba(0,0,0,0.06)']],
      },
    },
    axisTick: { show: false },
    splitLine: { show: false },
    axisLabel: { show: false },
    pointer: { show: false },
    detail: {
      valueAnimation: true,
      formatter: '{value}%',
      color: color.value,
      fontSize: 18,
      fontWeight: 300,
      offsetCenter: [0, '10%'],
    },
    data: [{ value: percent.value }],
  }],
}))
</script>

<template>
  <div class="budget-ring">
    <VChart :option="option" autoresize style="width:130px;height:130px;" />
    <div class="ring-label">预算消耗</div>
  </div>
</template>

<style scoped>
.budget-ring {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex-shrink: 0;
}
.ring-label {
  font-size: 10px;
  color: #888;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  margin-top: -10px;
}
</style>
