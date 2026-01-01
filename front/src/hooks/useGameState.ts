import { useStorage } from '@vueuse/core';
import { nextTick, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import type { Ending, MovieTemplate } from '../types/movie';

/**
 * 游戏状态管理 Hook
 * 状态持久化在 localStorage 中，各组件直接调用此 hook 获取相同的数据
 */
export function useGameState() {
  const router = useRouter();

  // 持久化游戏数据
  const gameData = useStorage<MovieTemplate | null>(
    'mg_active_game_data',
    null,
    localStorage,
    {
      serializer: {
        read: (v: unknown) => {
          if (
            !v ||
            v === 'null' ||
            v === 'undefined' ||
            v === '[object Object]'
          )
            return null;
          try {
            return JSON.parse(String(v));
          } catch (e) {
            console.warn('Failed to parse game data, resetting:', e);
            return null;
          }
        },
        write: (v: unknown) => JSON.stringify(v),
      },
    },
  );

  const endingData = useStorage<Ending | null>(
    'mg_ending',
    null,
    localStorage,
    {
      serializer: {
        read: (v: unknown) => {
          if (
            !v ||
            v === 'null' ||
            v === 'undefined' ||
            v === '[object Object]'
          )
            return null;
          try {
            return JSON.parse(String(v));
          } catch (e) {
            console.warn('Failed to parse ending data, resetting:', e);
            return null;
          }
        },
        write: (v: unknown) => JSON.stringify(v),
      },
    },
  );

  // 检查损坏的存储
  onMounted(() => {
    // @ts-expect-error - 需要检查字符串类型
    if (gameData.value === '[object Object]') {
      console.warn('Fixing corrupted game data storage');
      gameData.value = null;
    }
    // @ts-expect-error - 需要检查字符串类型
    if (endingData.value === '[object Object]') {
      console.warn('Fixing corrupted ending data storage');
      endingData.value = null;
    }
  });

  const loadGameData = async (
    data: MovieTemplate,
    entry: 'owner' | 'import' = 'owner',
    path = '/game',
  ) => {
    sessionStorage.setItem('mg_play_entry', entry);

    localStorage.removeItem('mg_current_node');
    localStorage.removeItem('mg_player_state');
    localStorage.removeItem('mg_history_stack');
    localStorage.removeItem('mg_affinity_state');

    endingData.value = null;

    const persistedData =
      entry === 'import'
        ? (() => {
            const cloned = JSON.parse(JSON.stringify(data)) as MovieTemplate;
            delete cloned.requestId;
            return cloned;
          })()
        : data;

    localStorage.setItem('mg_active_game_data', JSON.stringify(persistedData));

    gameData.value = null;
    await nextTick();
    gameData.value = persistedData;
    await nextTick();

    router.push(path);
  };

  /**
   * 开始新游戏
   */
  const handleGameStart = async (data: MovieTemplate) => {
    await loadGameData(data, 'owner', '/game');
  };

  /**
   * 游戏结束
   */
  const handleGameEnd = (ending: Ending) => {
    endingData.value = ending;
    router.push('/ending');
  };

  /**
   * 重新开始当前游戏
   */
  const handleRestartPlay = () => {
    localStorage.removeItem('mg_current_node');
    localStorage.removeItem('mg_player_state');
    localStorage.removeItem('mg_history_stack');
    localStorage.removeItem('mg_affinity_state');
    endingData.value = null;
    router.push('/game');
  };

  /**
   * 返回首页重新制作
   */
  const handleRemake = () => {
    gameData.value = null;
    endingData.value = null;
    localStorage.removeItem('mg_current_node');
    localStorage.removeItem('mg_player_state');
    localStorage.removeItem('mg_history_stack');
    localStorage.removeItem('mg_affinity_state');
    router.push('/');
  };

  return {
    gameData,
    endingData,
    loadGameData,
    handleGameStart,
    handleGameEnd,
    handleRestartPlay,
    handleRemake,
  };
}
