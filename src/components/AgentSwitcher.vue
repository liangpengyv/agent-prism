<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import type { AgentId, AgentInfo } from '../composables/useAgentSwitch'

defineProps<{
  currentAgent: AgentId
  agents: AgentInfo[]
}>()

const emit = defineEmits<{
  change: [agent: AgentId]
}>()

const isOpen = ref(false)

function toggle(e: Event) {
  e.stopPropagation()
  isOpen.value = !isOpen.value
}

function select(agent: AgentId, e: Event) {
  e.stopPropagation()
  emit('change', agent)
  isOpen.value = false
}

function handleClickOutside() {
  isOpen.value = false
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<template>
  <div class="agent-switcher">
    <button class="switcher-btn" @click.stop="toggle">
      {{ agents.find(a => a.id === currentAgent)?.label || 'Agent' }} ▾
    </button>
    <div v-if="isOpen" class="dropdown">
      <div
        v-for="agent in agents"
        :key="agent.id"
        class="dropdown-item"
        @click.stop="select(agent.id, $event)"
      >
        <span class="checkmark">{{ currentAgent === agent.id ? '✓' : '' }}</span>
        <span>{{ agent.label }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.agent-switcher {
  position: relative;
}

.switcher-btn {
  background: none;
  border: none;
  font-size: 14px;
  font-weight: 500;
  letter-spacing: 0.08em;
  color: #333;
  cursor: pointer;
  padding: 0;
  display: flex;
  align-items: center;
  gap: 4px;
}

.switcher-btn:hover {
  color: #0077cc;
}

.dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  margin-top: 4px;
  background: white;
  border: 1px solid #e0e0e0;
  border-radius: 6px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  min-width: 140px;
  z-index: 1000;
}

.dropdown-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s;
}

.dropdown-item:first-child {
  border-radius: 6px 6px 0 0;
}

.dropdown-item:last-child {
  border-radius: 0 0 6px 6px;
}

.dropdown-item:hover {
  background: #f5f5f5;
}

.checkmark {
  width: 14px;
  text-align: center;
  font-size: 12px;
}
</style>
