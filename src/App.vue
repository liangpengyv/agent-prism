<!-- src/App.vue -->
<script setup lang="ts">
import Dashboard from './views/Dashboard.vue'
</script>

<template>
  <Dashboard />
</template>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
html {
  width: 100%;
  height: 100%;
  /* clip-path 让整个 WebView 内容按圆角裁切，圆角外完全透明 */
  clip-path: inset(0 round 12px);
  overflow: hidden;
  background: transparent;
}
body {
  width: 100%;
  height: 100%;
  background: transparent;
  overflow: hidden;
}
#app {
  width: 100%;
  height: 100%;
  background: rgba(20, 20, 22, 0.88);
  overflow: hidden;
}
</style>
