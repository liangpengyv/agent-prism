<!-- src/App.vue -->
<script setup lang="ts">
import { ref } from 'vue'
import Dashboard from './views/Dashboard.vue'
import Settings from './views/Settings.vue'

const page = ref<'dashboard' | 'settings'>('dashboard')
</script>

<template>
  <Dashboard v-if="page === 'dashboard'" @openSettings="page = 'settings'" />
  <Settings v-else @back="page = 'dashboard'" />
</template>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
html, body, #app {
  width: 100%;
  height: 100%;
  overflow: hidden;
}
</style>
