<!-- src/views/Settings.vue -->
<script setup lang="ts">
import { ref, onMounted, reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-opener'
import type { CommandResult } from '../composables/useStats'

defineEmits<{ back: [] }>()

interface ModelPrice {
  input_per_1m: number
  cached_input_per_1m: number
  output_per_1m: number
}

type PriceMap = Record<string, ModelPrice>

const DEFAULT_BUDGET = 10_000_000

// 版本与更新
const appVersion = ref('')
const checking = ref(false)
const updateMsg = ref<{ text: string; url?: string } | null>(null)

// 预算
const budgetInput = ref<string>('')
const savingBudget = ref(false)
const budgetMsg = ref<string | null>(null)

// 价格表
const prices = ref<PriceMap>({})
const savingPrices = ref(false)
const resettingPrices = ref(false)
const pricesMsg = ref<string | null>(null)

// 新增表单
const newModel = reactive({ name: '', input_per_1m: '', cached_input_per_1m: '', output_per_1m: '' })
const addError = ref<string | null>(null)

// 编辑状态：key = model name
const editing = ref<string | null>(null)
const editBuf = reactive<ModelPrice>({ input_per_1m: 0, cached_input_per_1m: 0, output_per_1m: 0 })

onMounted(async () => {
  const [vRes, bRes, pRes] = await Promise.all([
    invoke<CommandResult<string>>('get_app_version'),
    invoke<CommandResult<number | null>>('get_budget'),
    invoke<CommandResult<PriceMap>>('get_prices'),
  ])
  if (vRes.data) appVersion.value = vRes.data
  budgetInput.value = String(bRes.data ?? DEFAULT_BUDGET)
  if (pRes.data) prices.value = pRes.data
})

interface UpdateInfo { has_update: boolean; latest_version: string; release_url: string }

async function checkUpdate() {
  checking.value = true
  updateMsg.value = null
  try {
    const res = await invoke<CommandResult<UpdateInfo>>('check_update')
    if (res.error) {
      updateMsg.value = { text: `检查失败：${res.error}` }
    } else if (res.data?.has_update) {
      updateMsg.value = { text: `发现新版本 v${res.data.latest_version}`, url: res.data.release_url }
    } else {
      updateMsg.value = { text: '当前已是最新版本' }
    }
  } finally {
    checking.value = false
  }
}

async function saveBudget() {
  const val = parseInt(budgetInput.value, 10)
  if (isNaN(val) || val <= 0) return
  savingBudget.value = true
  budgetMsg.value = null
  try {
    await invoke('set_budget', { tokens: val })
    budgetMsg.value = '已保存'
    setTimeout(() => { budgetMsg.value = null }, 2000)
  } finally {
    savingBudget.value = false
  }
}

async function savePrices() {
  savingPrices.value = true
  pricesMsg.value = null
  try {
    await invoke('set_prices', { prices: prices.value })
    pricesMsg.value = '已保存'
    setTimeout(() => { pricesMsg.value = null }, 2000)
  } finally {
    savingPrices.value = false
  }
}

async function resetPrices() {
  resettingPrices.value = true
  pricesMsg.value = null
  try {
    const res = await invoke<CommandResult<PriceMap>>('reset_prices')
    if (res.data) prices.value = res.data
    pricesMsg.value = '已恢复预设'
    setTimeout(() => { pricesMsg.value = null }, 2000)
  } finally {
    resettingPrices.value = false
  }
}

function startEdit(name: string) {
  editing.value = name
  const p = prices.value[name]
  editBuf.input_per_1m = p.input_per_1m
  editBuf.cached_input_per_1m = p.cached_input_per_1m
  editBuf.output_per_1m = p.output_per_1m
}

function confirmEdit() {
  if (!editing.value) return
  prices.value[editing.value] = { ...editBuf }
  editing.value = null
}

function cancelEdit() {
  editing.value = null
}

function deleteModel(name: string) {
  delete prices.value[name]
  if (editing.value === name) editing.value = null
}

function addModel() {
  addError.value = null
  const name = newModel.name.trim()
  if (!name) { addError.value = '模型名称不能为空'; return }
  if (prices.value[name]) { addError.value = '模型名已存在'; return }
  const inp = parseFloat(newModel.input_per_1m)
  const cac = parseFloat(newModel.cached_input_per_1m)
  const out = parseFloat(newModel.output_per_1m)
  if (isNaN(inp) || isNaN(cac) || isNaN(out)) { addError.value = '价格必须为数字'; return }
  prices.value[name] = { input_per_1m: inp, cached_input_per_1m: cac, output_per_1m: out }
  newModel.name = ''; newModel.input_per_1m = ''; newModel.cached_input_per_1m = ''; newModel.output_per_1m = ''
}

const sortedModels = () => Object.keys(prices.value).sort()
</script>

<template>
  <div class="settings">
    <header class="header">
      <button class="back-btn" @click="$emit('back')">← 返回</button>
      <span class="title">设置</span>
    </header>

    <!-- 关于 -->
    <div class="section">
      <div class="section-title">关于</div>
      <div class="about-row">
        <span class="about-version">AgentPrism {{ appVersion ? `v${appVersion}` : '' }}</span>
        <div class="about-actions">
          <span v-if="updateMsg" class="update-msg" :class="{ 'has-update': updateMsg.url }">
            {{ updateMsg.text }}
            <a v-if="updateMsg.url" @click.prevent="open(updateMsg.url!)" href="#">前往下载</a>
          </span>
          <button class="btn-secondary" @click="checkUpdate" :disabled="checking">
            {{ checking ? '检查中…' : '检查更新' }}
          </button>
        </div>
      </div>
    </div>

    <!-- 预算 -->
    <div class="section">
      <div class="section-title">预算管理</div>
      <div class="field-label">月度 Token 预算上限</div>
      <div class="field-row">
        <input v-model="budgetInput" type="number" min="1" class="field-input" placeholder="例：10000000" />
        <button class="btn-primary" @click="saveBudget" :disabled="savingBudget">
          {{ savingBudget ? '保存中…' : '保存' }}
        </button>
      </div>
      <div class="field-hint">用于圆环预算图的上限基准（单位：token）</div>
      <div v-if="budgetMsg" class="save-msg">{{ budgetMsg }}</div>
    </div>

    <!-- 计费价格表 -->
    <div class="section">
      <div class="section-title-row">
        <span class="section-title">计费价格表（/1M token，单位：$）</span>
        <div class="section-actions">
          <span v-if="pricesMsg" class="save-msg">{{ pricesMsg }}</span>
          <button class="btn-secondary" @click="resetPrices" :disabled="resettingPrices">
            {{ resettingPrices ? '重置中…' : '恢复预设' }}
          </button>
          <button class="btn-primary" @click="savePrices" :disabled="savingPrices">
            {{ savingPrices ? '保存中…' : '保存价格表' }}
          </button>
        </div>
      </div>

      <div class="price-note">所有费用均为估算，非真实账单</div>

      <table class="price-table">
        <thead>
          <tr>
            <th>模型名称</th>
            <th>输入</th>
            <th>缓存输入</th>
            <th>输出</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <template v-for="name in sortedModels()" :key="name">
            <!-- 编辑行 -->
            <tr v-if="editing === name" class="edit-row">
              <td class="model-name-cell">{{ name }}</td>
              <td><input v-model.number="editBuf.input_per_1m" type="number" step="0.001" class="price-input" /></td>
              <td><input v-model.number="editBuf.cached_input_per_1m" type="number" step="0.001" class="price-input" /></td>
              <td><input v-model.number="editBuf.output_per_1m" type="number" step="0.001" class="price-input" /></td>
              <td class="actions-cell">
                <button class="btn-sm btn-ok" @click="confirmEdit">确定</button>
                <button class="btn-sm btn-cancel" @click="cancelEdit">取消</button>
              </td>
            </tr>
            <!-- 普通行 -->
            <tr v-else>
              <td class="model-name-cell">{{ name }}</td>
              <td>${{ prices[name].input_per_1m }}</td>
              <td>${{ prices[name].cached_input_per_1m }}</td>
              <td>${{ prices[name].output_per_1m }}</td>
              <td class="actions-cell">
                <button class="btn-sm btn-edit" @click="startEdit(name)">编辑</button>
                <button class="btn-sm btn-del" @click="deleteModel(name)">删除</button>
              </td>
            </tr>
          </template>

          <!-- 新增行 -->
          <tr class="add-row">
            <td><input v-model="newModel.name" class="price-input" placeholder="模型名" /></td>
            <td><input v-model="newModel.input_per_1m" type="number" step="0.001" class="price-input" placeholder="输入" /></td>
            <td><input v-model="newModel.cached_input_per_1m" type="number" step="0.001" class="price-input" placeholder="缓存" /></td>
            <td><input v-model="newModel.output_per_1m" type="number" step="0.001" class="price-input" placeholder="输出" /></td>
            <td class="actions-cell">
              <button class="btn-sm btn-ok" @click="addModel">添加</button>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-if="addError" class="add-error">{{ addError }}</div>
    </div>
  </div>
</template>

<style scoped>
.settings { display: flex; flex-direction: column; height: 100vh; font-family: -apple-system, sans-serif; color: #333; overflow-y: auto; }
.header { display: flex; align-items: center; gap: 12px; padding: 10px 20px; border-bottom: 1px solid #e0e0e0; flex-shrink: 0; }
.back-btn { background: none; border: none; color: #0077cc; font-size: 13px; cursor: pointer; padding: 0; }
.back-btn:hover { text-decoration: underline; }
.title { font-size: 14px; font-weight: 500; }
.section { padding: 16px 20px; border-bottom: 1px solid #f0f0f0; }
.section-title { font-size: 11px; color: #888; text-transform: uppercase; letter-spacing: 0.06em; }
.section-title-row { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
.section-actions { display: flex; align-items: center; gap: 10px; }
.field-label { font-size: 13px; color: #333; margin-bottom: 8px; }
.field-row { display: flex; gap: 8px; align-items: center; margin-bottom: 6px; }
.field-input { flex: 1; padding: 6px 10px; border: 1px solid #ccc; border-radius: 5px; font-size: 13px; color: #333; }
.field-hint { font-size: 11px; color: #aaa; }
.save-msg { font-size: 12px; color: #4CAF50; }
.price-note { font-size: 11px; color: #aaa; margin-bottom: 10px; }
.btn-primary { background: #0077cc; border: none; border-radius: 5px; color: #fff; font-size: 12px; padding: 6px 14px; cursor: pointer; white-space: nowrap; }
.btn-primary:hover { background: #005fa3; }
.btn-primary:disabled { opacity: 0.5; cursor: default; }
.btn-secondary { background: #f0f0f0; border: none; border-radius: 5px; color: #555; font-size: 12px; padding: 6px 14px; cursor: pointer; white-space: nowrap; }
.btn-secondary:hover { background: #e0e0e0; }
.btn-secondary:disabled { opacity: 0.5; cursor: default; }
.price-table { width: 100%; border-collapse: collapse; font-size: 12px; }
.price-table th { text-align: left; padding: 6px 8px; color: #888; font-weight: 400; border-bottom: 1px solid #e0e0e0; }
.price-table td { padding: 5px 8px; border-bottom: 1px solid #f5f5f5; color: #333; }
.model-name-cell { font-weight: 500; color: #333; max-width: 140px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.price-input { width: 80px; padding: 3px 6px; border: 1px solid #ccc; border-radius: 4px; font-size: 12px; }
.add-row td { background: #fafafa; }
.edit-row td { background: #fffbf0; }
.actions-cell { white-space: nowrap; }
.btn-sm { font-size: 11px; padding: 3px 8px; border: none; border-radius: 4px; cursor: pointer; margin-right: 4px; }
.btn-edit { background: #e8f0fe; color: #0077cc; }
.btn-edit:hover { background: #d0e4ff; }
.btn-del { background: #fce4e4; color: #c0392b; }
.btn-del:hover { background: #f5b7b1; }
.btn-ok { background: #e8f5e9; color: #2e7d32; }
.btn-ok:hover { background: #c8e6c9; }
.btn-cancel { background: #f5f5f5; color: #666; }
.btn-cancel:hover { background: #e0e0e0; }
.add-error { font-size: 12px; color: #c0392b; margin-top: 6px; }
.about-row { display: flex; justify-content: space-between; align-items: center; margin-top: 8px; }
.about-version { font-size: 13px; color: #555; }
.about-actions { display: flex; align-items: center; gap: 10px; }
.update-msg { font-size: 12px; color: #888; }
.update-msg.has-update { color: #2e7d32; }
.update-msg a { color: #0077cc; text-decoration: none; margin-left: 4px; cursor: pointer; }
.update-msg a:hover { text-decoration: underline; }
</style>
