<!-- src/App.vue -->
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import Dashboard from './views/Dashboard.vue'
import Settings from './views/Settings.vue'
import { useAgentSwitch } from './composables/useAgentSwitch'
import type { AgentId } from './composables/useAgentSwitch'

const page = ref<'dashboard' | 'settings'>('dashboard')
const { currentAgent, init, switchAgent } = useAgentSwitch()

onMounted(async () => {
  await init()
})

async function handleAgentChange(agent: AgentId) {
  await switchAgent(agent)
}
</script>

<template>
  <Dashboard
    v-if="page === 'dashboard'"
    :currentAgent="currentAgent"
    @openSettings="page = 'settings'"
    @agentChange="handleAgentChange"
  />
  <Settings
    v-else
    :currentAgent="currentAgent"
    @back="page = 'dashboard'"
  />
</template>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
html, body, #app {
  width: 100%;
  height: 100%;
  overflow: hidden;
}
</style>
