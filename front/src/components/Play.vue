<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { getSharedGame } from '../api';
import { db } from '../utils/db';
import { useGameState } from '../hooks/useGameState';
import type { MovieTemplate } from '../types/movie';
import Game from './Game.vue';
import CinematicLoader from './ui/CinematicLoader.vue';

const route = useRoute();
const router = useRouter();
const { gameData } = useGameState();

const error = ref('');
const isLoading = ref(true);
const gameLoaded = ref(false);

onMounted(async () => {
  const id = route.params.id as string;
  if (!id) {
    error.value = '无效的游戏 ID';
    isLoading.value = false;
    return;
  }

  sessionStorage.setItem('mg_shared_play_id', id);

  // Check if we are playing as owner (local debug) or shared
  const rawEntry = String(sessionStorage.getItem('mg_play_entry') || '').trim();
  const ownerPlayId = String(
    sessionStorage.getItem('mg_owner_play_id') || '',
  ).trim();

  let entryType: 'owner' | 'shared' = 'shared';
  if (rawEntry === 'owner' && ownerPlayId === id) {
      entryType = 'owner';
  } else {
    sessionStorage.setItem('mg_owner_play_id', '');
    sessionStorage.setItem('mg_play_entry', 'shared');
  }

  try {
    let data: MovieTemplate | null = null;
    
    // 1. Try local
    const local = await db.getPlayedGame(id);
    if (local) {
        data = local as unknown as MovieTemplate;
    }
    
    // 2. If shared or missing, try API
    // Re-request shared games requirement
    if (!data || (local?.source === 'shared' && entryType === 'shared')) {
        try {
            const remote = await getSharedGame(id);
            await db.savePlayedGame(remote, entryType);
            data = remote as unknown as MovieTemplate;
        } catch (apiErr) {
            console.warn('Failed to fetch from API, using local if available', apiErr);
            if (!data) throw apiErr;
        }
    }

    if (!data) {
      throw new Error('未找到游戏数据');
    }

    // Initialize Active Session
    await db.setActiveSession({
        gameId: id,
        currentNodeId: 'start', // Default
        playerState: { flags: {}, variables: {} },
        historyStack: [],
    });
    
    // Check if we have an existing session for this game?
    // If we are "Replaying" or "Resuming", we might want to keep it.
    // But Play.vue usually implies "Start" or "Load specific ID".
    // If we just clicked a link, we probably want to start fresh OR resume?
    // The user requirement: "save played games ... mark one as active".
    // If I reload the page, I want to resume.
    // If I click a link, maybe resume if same ID?
    // Current logic resets session.
    // I'll keep it as "Reset" for now to ensure clean start, unless we check DB for existing session with same ID.
    // Actually, `Game.vue` handles restoring session.
    // If I overwrite session here, I lose progress.
    // So I should check if active session matches this ID.
    const currentSession = await db.getActiveSession();
    if (currentSession && currentSession.gameId === id) {
        // Resume
    } else {
        // New session
        await db.setActiveSession({
            gameId: id,
            currentNodeId: 'start',
            playerState: { flags: {}, variables: {} },
            historyStack: [],
        });
    }

    gameData.value = data;
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
