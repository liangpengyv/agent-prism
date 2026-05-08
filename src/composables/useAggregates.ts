import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CommandResult } from './useStats'

export interface ProjectStat {
  project: string
  tokens: number
  cost_usd: number
}

export interface ModelStat {
  model: string
  tokens: number
  cost_usd: number
}

export interface DayStat {
  date: string
  tokens: number
  cost_usd: number
}

export function useAggregates() {
  const byProject = ref<ProjectStat[]>([])
  const byModel = ref<ModelStat[]>([])
  const byDate = ref<DayStat[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function loadAll() {
    loading.value = true
    error.value = null
    try {
      const [pRes, mRes, dRes] = await Promise.all([
        invoke<CommandResult<ProjectStat[]>>('get_by_project'),
        invoke<CommandResult<ModelStat[]>>('get_by_model'),
        invoke<CommandResult<DayStat[]>>('get_by_date'),
      ])
      if (pRes.error) error.value = pRes.error
      else byProject.value = pRes.data ?? []
      if (!mRes.error) byModel.value = mRes.data ?? []
      if (!dRes.error) byDate.value = dRes.data ?? []
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  return { byProject, byModel, byDate, loading, error, loadAll }
}
