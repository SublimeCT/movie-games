<script setup lang="ts">
import { useStorage } from '@vueuse/core';
import { onMounted } from 'vue';
import { useRouter } from 'vue-router';
import type { Ending, MovieTemplate } from './types/movie';

const router = useRouter();

// Persist the active game data with custom serializer to prevent [object Object] corruption
const gameData = useStorage<MovieTemplate | null>(
  'mg_active_game_data',
  null,
  localStorage,
  {
    serializer: {
      // biome-ignore lint/suspicious/noExplicitAny: Serializer needs flexibility
      read: (v: any) => {
        if (!v || v === 'null' || v === 'undefined' || v === '[object Object]')
          return null;
        try {
          return JSON.parse(v);
        } catch (e) {
          console.warn('Failed to parse game data, resetting:', e);
          return null;
        }
      },
      // biome-ignore lint/suspicious/noExplicitAny: Serializer needs flexibility
      write: (v: any) => JSON.stringify(v),
    },
  },
);
const endingData = useStorage<Ending | null>('mg_ending', null, localStorage, {
  serializer: {
    // biome-ignore lint/suspicious/noExplicitAny: Serializer needs flexibility
    read: (v: any) => {
      if (!v || v === 'null' || v === 'undefined' || v === '[object Object]')
        return null;
      try {
        return JSON.parse(v);
      } catch (e) {
        console.warn('Failed to parse ending data, resetting:', e);
        return null;
      }
    },
    // biome-ignore lint/suspicious/noExplicitAny: Serializer needs flexibility
    write: (v: any) => JSON.stringify(v),
  },
});

// Check for corrupted storage
onMounted(() => {
  // Double check in case the serializer didn't catch it initially (unlikely with custom serializer but safe)
  // @ts-expect-error
  if (gameData.value === '[object Object]') {
    console.warn('Fixing corrupted game data storage (manual check)');
    gameData.value = null;
  }

  // @ts-expect-error
  if (endingData.value === '[object Object]') {
    console.warn('Fixing corrupted ending data storage (manual check)');
    endingData.value = null;
  }
});

const handleGameStart = (data: MovieTemplate) => {
  // Ensure fresh start for new game
  localStorage.removeItem('mg_current_node');
  localStorage.removeItem('mg_player_state');

  gameData.value = data;
  router.push('/game');
};

const handleGameEnd = (ending: Ending) => {
  endingData.value = ending;
  router.push('/ending');
};

const handleRestartPlay = () => {
  localStorage.removeItem('mg_current_node');
  localStorage.removeItem('mg_player_state');
  localStorage.removeItem('mg_history_stack');
  endingData.value = null;
  router.push('/game');
};

const handleRemake = () => {
  gameData.value = null;
  endingData.value = null;
  localStorage.removeItem('mg_current_node');
  localStorage.removeItem('mg_player_state');
  localStorage.removeItem('mg_history_stack');
  router.push('/');
};
</script>

<template>
  <div class="min-h-screen bg-neutral-950 text-neutral-100 font-sans selection:bg-purple-500/30">
    <router-view 
      :data="gameData" 
      :ending="endingData"
      @start="handleGameStart" 
      @end="handleGameEnd" 
      @restart-play="handleRestartPlay"
      @remake="handleRemake"
    />
  </div>
</template>
