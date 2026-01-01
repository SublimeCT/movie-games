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

  /**
   * 首页“角色阵容”的本地输入结构（轻量版），用于从模板回填到本地存储。
   */
  type CharacterInputLite = {
    name: string;
    description: string;
    gender: string;
    isMain: boolean;
    avatarPath?: string;
  };

  /**
   * 将模板 meta.genre 的字符串解析为可用于本地存储的标签数组。
   */
  const parseGenreList = (genre: string): string[] => {
    const raw = String(genre || '').trim();
    if (!raw) return [];

    return Array.from(
      new Set(
        raw
          .split(/\s*(?:\/|\||,|，|、|;|；)\s*/g)
          .map((x) => x.trim())
          .filter(Boolean),
      ),
    );
  };

  /**
   * 根据模板角色的 key/name/role 估算“主角”优先级。
   */
  const scoreTemplateCharacter = (
    key: string,
    c: MovieTemplate['characters'][string],
  ) => {
    const k = String(key || '').toLowerCase();
    const name = String(c?.name || '').toLowerCase();
    const role = String(c?.role || '').toLowerCase();

    let score = 0;
    if (/player|protagonist|main/.test(k)) score += 5;
    if (name.includes('主角') || name === '我') score += 6;
    if (role.includes('主角') || role.includes('protagonist')) score += 3;
    if (typeof c?.age === 'number' && c.age > 0) score += 1;

    return score;
  };

  /**
   * 导入 JSON 进入游玩/设计时，将模板中的主题/简介/类型/角色回填到本地向导存储。
   */
  const persistHomeInputsFromTemplate = (template: MovieTemplate) => {
    try {
      const theme = String(
        template.meta?.logline || template.title || '',
      ).trim();
      const synopsis = String(template.meta?.synopsis || '').trim();
      const genres = parseGenreList(String(template.meta?.genre || ''));

      const entries = Object.entries(template.characters || {});
      const scored = entries
        .map(([key, c]) => ({ key, c, score: scoreTemplateCharacter(key, c) }))
        .sort((a, b) => b.score - a.score);

      const mainKey = scored[0]?.key ?? '';

      const chars: CharacterInputLite[] = entries
        .map(([key, c]) => {
          const name = String(c?.name || '').trim();
          const gender = String(c?.gender || '其他').trim() || '其他';
          const descriptionParts = [
            String(c?.role || '').trim(),
            String(c?.background || '').trim(),
          ].filter(Boolean);
          const description = descriptionParts.join('，');

          return {
            name: name || '角色',
            gender,
            description,
            isMain: key === mainKey,
            avatarPath: c?.avatarPath || undefined,
          };
        })
        .filter((c) => Boolean(String(c.name || '').trim()));

      if (chars.length === 0) {
        chars.push({
          name: '主角',
          description: '故事的核心人物',
          gender: '男',
          isMain: true,
        });
      } else if (!chars.some((c) => c.isMain)) {
        const first = chars[0];
        if (first) chars[0] = { ...first, isMain: true };
      }

      localStorage.setItem('mg_theme', theme);
      localStorage.setItem('mg_synopsis', synopsis);
      localStorage.setItem('mg_genres', JSON.stringify(genres));
      localStorage.setItem('mg_characters', JSON.stringify(chars));
    } catch {}
  };

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

    if (entry === 'import') {
      persistHomeInputsFromTemplate(persistedData);
    }

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
