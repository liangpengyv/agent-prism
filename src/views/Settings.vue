<!-- src/views/Settings.vue -->
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CommandResult } from '../composables/useStats'

defineEmits<{ back: [] }>()

const DEFAULT_BUDGET = 10_000_000

const budgetInput = ref<string>('')
const saving = ref(false)
const saveMsg = ref<string | null>(null)

onMounted(async () => {
  const res = await invoke<CommandResult<number | null>>('get_budget')
  budgetInput.value = String(res.data ?? DEFAULT_BUDGET)
})

async function save() {
  const val = parseInt(budgetInput.value, 10)
  if (isNaN(val) || val <= 0) return
  saving.value = true
  saveMsg.value = null
  try {
    await invoke('set_budget', { tokens: val })
    saveMsg.value = '已保存'
    setTimeout(() => { saveMsg.value = null }, 2000)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="settings">
    <header class="header">
      <button class="back-btn" @click="$emit('back')">← 返回</button>
      <span class="title">设置</span>
    </header>

    <div class="section">
      <div class="section-title">预算管理</div>
      <div class="field">
        <label class="field-label">月度 Token 预算上限</label>
        <div class="field-row">
          <input
            v-model="budgetInput"
            type="number"
            min="1"
            class="field-input"
            placeholder="例：10000000"
          />
          <button class="save-btn" @click="save" :disabled="saving">
            {{ saving ? '保存中…' : '保存' }}
          </button>
        </div>
        <div class="field-hint">用于圆环预算图的上限基准（单位：token）</div>
        <div v-if="saveMsg" class="save-msg">{{ saveMsg }}</div>
      </div>
    </div>

    <div class="section">
      <div class="section-title">计费价格表（内置，仅供参考）</div>
      <table class="price-table">
        <thead>
          <tr><th>模型</th><th>输入 /1M</th><th>缓存输入 /1M</th><th>输出 /1M</th></tr>
        </thead>
        <tbody>
          <tr><td>codex-mini</td><td>$1.50</td><td>$0.375</td><td>$6.00</td></tr>
          <tr><td>gpt-4.1</td><td>$2.00</td><td>$0.50</td><td>$8.00</td></tr>
          <tr><td>gpt-4.1-mini</td><td>$0.40</td><td>$0.10</td><td>$1.60</td></tr>
        </tbody>
      </table>
      <div class="price-note">所有费用均为估算，非真实账单</div>
    </div>
  </div>
</template>

<style scoped>
.settings { display: flex; flex-direction: column; height: 100vh; font-family: -apple-system, sans-serif; color: #333; }
.header { display: flex; align-items: center; gap: 12px; padding: 10px 20px; border-bottom: 1px solid #e0e0e0; }
.back-btn { background: none; border: none; color: #0077cc; font-size: 13px; cursor: pointer; padding: 0; }
.back-btn:hover { text-decoration: underline; }
.title { font-size: 14px; font-weight: 500; }
.section { padding: 20px; border-bottom: 1px solid #f0f0f0; }
.section-title { font-size: 11px; color: #888; text-transform: uppercase; letter-spacing: 0.06em; margin-bottom: 14px; }
.field-label { font-size: 13px; color: #333; display: block; margin-bottom: 8px; }
.field-row { display: flex; gap: 8px; align-items: center; }
.field-input { flex: 1; padding: 6px 10px; border: 1px solid #ccc; border-radius: 5px; font-size: 13px; color: #333; }
.field-hint { font-size: 11px; color: #aaa; margin-top: 6px; }
.save-btn { background: #0077cc; border: none; border-radius: 5px; color: #fff; font-size: 12px; padding: 6px 14px; cursor: pointer; }
.save-btn:hover { background: #005fa3; }
.save-btn:disabled { opacity: 0.5; }
.save-msg { font-size: 12px; color: #4CAF50; margin-top: 6px; }
.price-table { width: 100%; border-collapse: collapse; font-size: 12px; }
.price-table th { text-align: left; padding: 6px 8px; color: #888; font-weight: 400; border-bottom: 1px solid #e0e0e0; }
.price-table td { padding: 6px 8px; border-bottom: 1px solid #f5f5f5; color: #333; }
.price-note { font-size: 11px; color: #aaa; margin-top: 10px; }
</style>
