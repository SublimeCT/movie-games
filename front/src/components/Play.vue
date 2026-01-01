<script setup lang="ts">
import { useStorage } from '@vueuse/core';
import { nextTick, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { getSharedGame } from '../api';
import Game from './Game.vue';
import CinematicLoader from './ui/CinematicLoader.vue';

const route = useRoute();
const router = useRouter();

const error = ref('');
const isLoading = ref(true);
const gameLoaded = ref(false);

// Use the same localStorage keys as useGameState for consistency
const gameDataStorage = useStorage('mg_active_game_data', null, localStorage, {
  serializer: {
    read: (v: unknown) => {
      if (!v || v === 'null' || v === 'undefined' || v === '[object Object]')
        return null;
      try {
        return JSON.parse(String(v));
      } catch {
        return null;
      }
    },
    write: (v: unknown) => JSON.stringify(v),
  },
});

onMounted(async () => {
  const id = route.params.id as string;
  if (!id) {
    error.value = '无效的游戏 ID';
    isLoading.value = false;
    return;
  }

  if (!sessionStorage.getItem('mg_play_entry')) {
    sessionStorage.setItem('mg_play_entry', 'shared');
  }

  try {
    const data = await getSharedGame(id);
    if (!data) {
      throw new Error('未找到游戏数据');
    }

    // Clear old state
    localStorage.removeItem('mg_current_node');
    localStorage.removeItem('mg_player_state');
    localStorage.removeItem('mg_history_stack');
    localStorage.removeItem('mg_ending');

    // Set game data directly to localStorage (without navigation)
    gameDataStorage.value = null;
    await nextTick();
    gameDataStorage.value = data;

    gameLoaded.value = true;
    isLoading.value = false;
  } catch (e: unknown) {
    console.error(e);
    error.value = e instanceof Error ? e.message : '加载游戏失败';
    isLoading.value = false;
    // 3秒后返回首页
    setTimeout(() => {
      router.push('/');
    }, 3000);
  }
});
</script>

<template>
  <div class="fixed inset-0 z-50 bg-neutral-950">
    <!-- Loading State -->
    <div v-if="isLoading" class="h-full flex items-center justify-center">
      <CinematicLoader text="正在载入剧本..." />
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="h-full flex items-center justify-center">
      <div class="text-center max-w-md px-6">
        <div class="text-red-500 text-xl font-bold mb-4">加载失败</div>
        <p class="text-neutral-400 mb-6">{{ error }}</p>
        <p class="text-neutral-600 text-sm">正在返回首页...</p>
      </div>
    </div>

    <!-- Game Content - directly render Game component without navigation -->
    <Game v-else-if="gameLoaded" />
  </div>
</template>
