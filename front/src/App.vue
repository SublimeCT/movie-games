<script setup lang="ts">
// App.vue 只负责初始化，不传递任何 props 或 events
// 各组件通过 useGameState hook 自行获取状态和方法
</script>

<template>
  <div class="min-h-[100dvh] w-full overflow-x-hidden bg-neutral-950 text-neutral-100 font-sans selection:bg-purple-500/30">
    <router-view />
  </div>
</template>
