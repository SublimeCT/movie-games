<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { getSharedGame } from '../api';
import type { MovieTemplate } from '../types/movie';
import CinematicLoader from './ui/CinematicLoader.vue';

const route = useRoute();
const router = useRouter();
const emit = defineEmits<{
  (e: 'start', data: MovieTemplate): void;
}>();

const error = ref('');
const isLoading = ref(true);

onMounted(async () => {
  const id = route.params.id as string;
  if (!id) {
    error.value = '无效的游戏 ID';
    isLoading.value = false;
    return;
  }

  try {
    const data = await getSharedGame(id);
    if (!data) {
      throw new Error('未找到游戏数据');
    }
    // 这里的 emit 'start' 会被 App.vue 捕获（如果 Play.vue 是直接在 router-view 中渲染的话）
    // 但是 Play.vue 是作为路由组件渲染的，App.vue 中的 <router-view @start="handleGameStart" /> 会处理这个事件
    emit('start', data);
  } catch (e: any) {
    console.error(e);
    error.value = e.message || '加载游戏失败';
    isLoading.value = false;
    // 3秒后返回首页
    setTimeout(() => {
      router.push('/');
    }, 3000);
  }
});
</script>

<template>
  <div class="fixed inset-0 z-50 bg-neutral-950 flex items-center justify-center">
    <div v-if="isLoading" class="text-center">
      <CinematicLoader text="正在载入剧本..." />
    </div>
    
    <div v-else-if="error" class="text-center max-w-md px-6">
      <div class="text-red-500 text-xl font-bold mb-4">加载失败</div>
      <p class="text-neutral-400 mb-6">{{ error }}</p>
      <p class="text-neutral-600 text-sm">正在返回首页...</p>
    </div>
  </div>
</template>
