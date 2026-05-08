<!-- src/App.vue -->
<script setup lang="ts">
import Dashboard from './views/Dashboard.vue'
</script>

<template>
  <Dashboard />
</template>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
html, body {
  background: transparent;
  height: 100%;
  overflow: hidden;
}
#app {
  height: 100%;
  border-radius: 12px;
  overflow: hidden;
  background: rgba(20, 20, 22, 0.82);
}
</style>
