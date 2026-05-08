<!-- src/components/ThreadList.vue -->
<script setup lang="ts">
import type { ThreadRecord } from '../composables/useStats'

defineProps<{ threads: ThreadRecord[] }>()

function shortPath(cwd: string): string {
  const parts = cwd.split('/')
  return parts.at(-1) || cwd
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return String(n)
}
</script>

<template>
  <div class="thread-list">
    <div v-if="threads.length === 0" class="empty">暂无线程数据</div>
    <div v-for="t in threads" :key="t.id" class="thread-item">
      <div class="thread-title">{{ t.title || '(无标题)' }}</div>
      <div class="thread-meta">
        <span class="project">{{ shortPath(t.cwd) }}</span>
        <span class="model">{{ t.model }}</span>
        <span class="tokens">{{ formatTokens(t.tokens_used) }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.thread-list { display: flex; flex-direction: column; gap: 1px; overflow-y: auto; height: 100%; }
.empty { color: #888; font-size: 13px; padding: 16px; text-align: center; }
.thread-item {
  padding: 10px 16px;
  border-bottom: 1px solid #e8e8e8;
  flex-shrink: 0;
}
.thread-item:hover { background: #f5f5f5; }
.thread-title { font-size: 13px; color: #222; margin-bottom: 4px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.thread-meta { display: flex; gap: 10px; font-size: 11px; color: #888; }
.project { color: #0077cc; }
.tokens { margin-left: auto; color: #555; }
</style>
