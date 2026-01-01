<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import ThreeDCard from './ThreeDCard.vue';
import Vortex from './vortex/Vortex.vue';

/**
 * List of cinematic loading phrases shown during generation.
 * cycled through to keep the user engaged.
 */
const phrases = [
  '正在构建宏大的故事世界...',
  '导演正在挑选最合适的演员...',
  '编剧正在推敲每一个剧情分支...',
  '摄影师正在寻找最佳镜头角度...',
  '灯光师正在布置场景氛围...',
  '音效师正在录制环境音...',
  '正在生成角色的详细背景...',
  '正在计算剧情的因果逻辑...',
  'AI 正在模拟数百万种可能的结局...',
  '正在渲染最终的剧本格式...',
  '最后检查逻辑一致性...',
  '马上就好，大片即将上映...',
];

const currentPhrase = ref(phrases[0]);
let intervalId: number;

/**
 * Start the phrase rotation on mount.
 * Updates every 8 seconds.
 */
onMounted(() => {
  let i = 0;
  intervalId = window.setInterval(() => {
    i = (i + 1) % phrases.length;
    currentPhrase.value = phrases[i];
  }, 8000); // Faster updates
});

/**
 * Clean up the interval on unmount.
 */
onUnmounted(() => {
  clearInterval(intervalId);
});
</script>

<template>
  <div class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm">
    <Vortex
        background-color="transparent"
        :particle-count="3000"
        :base-hue="260"
        :base-speed="1.5"
        :range-speed="0"
        :base-radius="2"
        :range-radius="0"
        class="flex flex-col items-center justify-center h-full w-full"
      >
        <ThreeDCard class="w-full max-w-md">
          <div class="relative w-full p-8 text-center space-y-8 backdrop-blur-sm rounded-3xl border border-white/10 bg-black/40">
            <div class="relative w-24 h-24 mx-auto">
              <svg viewBox="0 0 24 24" fill="none" class="w-full h-full text-purple-500 drop-shadow-[0_0_15px_rgba(168,85,247,0.5)] mg-rotate">
                <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2" class="opacity-20"/>
                <path d="M12 2C6.48 2 2 6.48 2 12C2 17.52 6.48 22 12 22C17.52 22 22 17.52 22 12C22 6.48 17.52 2 12 2ZM12 20C7.59 20 4 16.41 4 12C4 7.59 7.59 4 12 4C16.41 4 20 7.59 20 12C20 16.41 16.41 20 12 20Z" fill="currentColor" class="opacity-50"/>
                <circle cx="12" cy="12" r="3" fill="currentColor"/>
                <rect x="11" y="2" width="2" height="4" fill="currentColor"/>
                <rect x="11" y="18" width="2" height="4" fill="currentColor"/>
                <rect x="2" y="11" width="4" height="2" fill="currentColor"/>
                <rect x="18" y="11" width="4" height="2" fill="currentColor"/>
              </svg>
            </div>

            <!-- Text -->
            <div class="space-y-2">
              <h3 class="text-3xl font-black text-transparent bg-clip-text bg-gradient-to-r from-purple-400 via-pink-500 to-red-500 tracking-widest uppercase drop-shadow-sm">
                AI Director
              </h3>
              <p class="text-neutral-300 text-sm font-mono h-6 transition-all duration-500 ease-in-out tracking-wide">
                {{ currentPhrase }}
              </p>
            </div>

            <!-- Progress Line -->
            <div class="w-full h-1 bg-neutral-800 rounded-full overflow-hidden shadow-inner">
              <div class="h-full bg-gradient-to-r from-purple-600 via-pink-600 to-red-600 animate-progress origin-left shadow-[0_0_10px_rgba(236,72,153,0.5)]"></div>
            </div>
          </div>
        </ThreeDCard>
    </Vortex>
  </div>
</template>

<style scoped>
@keyframes progress {
  0% { transform: scaleX(0); }
  10% { transform: scaleX(0.1); }
  30% { transform: scaleX(0.4); }
  60% { transform: scaleX(0.7); }
  90% { transform: scaleX(0.95); }
  100% { transform: scaleX(0.98); }
}

.animate-progress {
  /* 150s duration as requested (2.5 minutes) */
  animation: progress 150s cubic-bezier(0.2, 0, 0.2, 1) forwards;
}

@keyframes mg-spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.mg-rotate {
  transform-origin: 50% 50%;
  animation: mg-spin 3.5s linear infinite;
}
</style>
