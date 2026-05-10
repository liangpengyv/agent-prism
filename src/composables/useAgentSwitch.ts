// src/composables/useAgentSwitch.ts
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CommandResult } from './useStats'

export type AgentId = 'codex' | 'claude-code'

export interface AgentInfo {
  id: AgentId
  label: string
}

export const AGENTS: AgentInfo[] = [
  { id: 'claude-code', label: 'Claude Code' },
  { id: 'codex', label: 'Codex' },
]

export function useAgentSwitch() {
  const currentAgent = ref<AgentId>('claude-code')

  async function init() {
    const res = await invoke<CommandResult<string | null>>('get_last_selected_agent')
    if (res.data === 'codex' || res.data === 'claude-code') {
      currentAgent.value = res.data
    }
  }

  async function switchAgent(agent: AgentId) {
    if (currentAgent.value === agent) return
    currentAgent.value = agent
    await invoke('set_last_selected_agent', { agent })
  }

  return { currentAgent, init, switchAgent, AGENTS }
}
