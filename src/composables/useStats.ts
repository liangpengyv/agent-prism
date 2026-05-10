// src/composables/useStats.ts
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface ReconcileResult {
  sqlite_total: number
  jsonl_total: number
  diff: number
  diff_rate: number
  warning: string | null
}

export interface SummaryData {
  total_tokens: number
  thread_count: number
  session_count: number
  estimated_cost_usd: number
  top_project: string | null
  reconcile: ReconcileResult
}

export interface ThreadRecord {
  id: string
  title: string
  cwd: string
  model: string
  model_provider: string
  tokens_used: number
  created_at: string
  updated_at: string
  source: string
}

export interface CommandResult<T> {
  data: T | null
  error: string | null
  warnings: string[]
}

export function useStats() {
  const summary = ref<SummaryData | null>(null)
  const threads = ref<ThreadRecord[]>([])
  const warnings = ref<string[]>([])
  const error = ref<string | null>(null)
  const loading = ref(false)

  async function loadSummary(agent: string) {
    loading.value = true
    error.value = null
    try {
      const result = await invoke<CommandResult<SummaryData>>('get_summary', { agent })
      if (result.error) {
        error.value = result.error
      } else {
        summary.value = result.data
        warnings.value = result.warnings
      }
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function loadThreads() {
    loading.value = true
    try {
      const result = await invoke<CommandResult<ThreadRecord[]>>('get_threads')
      if (result.error) {
        error.value = result.error
      } else {
        threads.value = result.data ?? []
        warnings.value = result.warnings
      }
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function refresh(agent: string) {
    await invoke('refresh')
    await loadSummary(agent)
    await loadThreads()
  }

  return { summary, threads, warnings, error, loading, loadSummary, loadThreads, refresh }
}

export function useDataUpdatedListener(callback: () => void): () => void {
  let unlisten: (() => void) | null = null
  listen('data-updated', () => callback()).then(fn => { unlisten = fn })
  return () => { if (unlisten) unlisten() }
}
