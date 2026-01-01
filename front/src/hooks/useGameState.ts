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

  const isRecord = (v: unknown): v is Record<string, unknown> =>
    Boolean(v) && typeof v === 'object';

  const createId = () => {
    try {
      return crypto.randomUUID();
    } catch {
      return `p_${Date.now()}_${Math.random().toString(16).slice(2)}`;
    }
  };

  const readLocalJson = <T>(key: string, fallback: T): T => {
    try {
      const raw = localStorage.getItem(key);
      if (!raw) return fallback;
      const v = JSON.parse(raw) as unknown;
      return v as T;
    } catch {
      return fallback;
    }
  };

  const readLocalTheme = () =>
    String(localStorage.getItem('mg_theme') || '').trim();
  const readLocalSynopsis = () =>
    String(localStorage.getItem('mg_synopsis') || '').trim();

  const readLocalGenres = (): string[] => {
    const v = readLocalJson<unknown>('mg_genres', []);
    if (!Array.isArray(v)) return [];
    return v.map((x) => String(x || '').trim()).filter(Boolean);
  };

  const readLocalCharacters = (): Array<{
    name: string;
    description: string;
    gender: string;
    isMain: boolean;
    avatarPath?: string;
  }> => {
    const v = readLocalJson<unknown>('mg_characters', []);
    if (!Array.isArray(v)) return [];
    return v
      .map((x) => {
        if (!isRecord(x)) return null;
        const name = String(x.name || '').trim();
        if (!name) return null;
        return {
          name,
          description: String(x.description || '').trim(),
          gender: String(x.gender || '其他').trim() || '其他',
          isMain: Boolean(x.isMain),
          avatarPath:
            typeof x.avatarPath === 'string' && x.avatarPath.trim()
              ? x.avatarPath.trim()
              : undefined,
        };
      })
      .filter(Boolean) as Array<{
      name: string;
      description: string;
      gender: string;
      isMain: boolean;
      avatarPath?: string;
    }>;
  };

  const normalizeTemplate = (input: unknown): MovieTemplate | null => {
    if (!isRecord(input)) return null;

    const nodes = input.nodes;
    if (!nodes || typeof nodes !== 'object') return null;

    const metaRaw = input.meta;
    const meta =
      metaRaw && typeof metaRaw === 'object'
        ? (metaRaw as Record<string, unknown>)
        : ({} as Record<string, unknown>);

    const localTheme = readLocalTheme();
    const localSynopsis = readLocalSynopsis();
    const localGenreList = readLocalGenres();
    const localChars = readLocalCharacters();

    const titleFromInput = String(input.title || '').trim();
    const loglineFromInput = String(meta.logline || '').trim();
    const theme = loglineFromInput || titleFromInput || localTheme;

    const synopsisFromInput = String(meta.synopsis || '').trim();
    const synopsis = synopsisFromInput || localSynopsis;

    const genreFromInput = String(meta.genre || '').trim();
    const genre = genreFromInput || localGenreList.filter(Boolean).join(' / ');

    const languageFromInput = String(meta.language || '').trim();
    const language =
      languageFromInput || String(navigator.language || '').trim() || 'zh-CN';

    const targetRuntimeMinutesRaw = meta.targetRuntimeMinutes;
    const targetRuntimeMinutes =
      typeof targetRuntimeMinutesRaw === 'number' &&
      Number.isFinite(targetRuntimeMinutesRaw)
        ? targetRuntimeMinutesRaw
        : 0;

    const charactersRaw = input.characters;
    const charactersFromInput =
      charactersRaw && typeof charactersRaw === 'object'
        ? (charactersRaw as MovieTemplate['characters'])
        : {};

    const buildCharactersFromLocal = () => {
      const list =
        localChars.length > 0
          ? localChars
          : [
              {
                name: '主角',
                description: '故事的核心人物',
                gender: '男',
                isMain: true,
              },
            ];

      const main = list.find((c) => c.isMain) ?? list[0];
      const map: MovieTemplate['characters'] = {};
      for (const c of list) {
        const id = String(c.name || '').trim() || createId();
        map[id] = {
          id,
          name: String(c.name || '').trim() || '角色',
          gender: String(c.gender || '其他').trim() || '其他',
          age: 0,
          role: String(c.description || '').trim(),
          background: '',
          avatarPath: c.avatarPath || undefined,
        };
      }

      if (main) {
        const mainId = String(main.name || '').trim();
        if (mainId && map[mainId]) {
          map[mainId] = { ...map[mainId], id: mainId };
        }
      }

      return map;
    };

    const characters =
      charactersFromInput && Object.keys(charactersFromInput).length > 0
        ? charactersFromInput
        : buildCharactersFromLocal();

    const provenanceRaw = input.provenance;
    const provenance =
      provenanceRaw && typeof provenanceRaw === 'object'
        ? (provenanceRaw as MovieTemplate['provenance'])
        : { createdBy: 'import', createdAt: new Date().toISOString() };

    const normalized: MovieTemplate = {
      requestId:
        typeof input.requestId === 'string' && input.requestId.trim()
          ? input.requestId.trim()
          : undefined,
      projectId:
        typeof input.projectId === 'string' && input.projectId.trim()
          ? input.projectId.trim()
          : createId(),
      title: theme || titleFromInput,
      version:
        typeof input.version === 'string' && input.version.trim()
          ? input.version.trim()
          : '1.0.0',
      owner:
        typeof input.owner === 'string' && input.owner.trim()
          ? input.owner.trim()
          : 'User',
      meta: {
        logline: theme,
        synopsis,
        targetRuntimeMinutes,
        genre,
        language,
      },
      backgroundImageBase64:
        typeof input.backgroundImageBase64 === 'string'
          ? input.backgroundImageBase64
          : undefined,
      nodes: nodes as MovieTemplate['nodes'],
      endings:
        input.endings && typeof input.endings === 'object'
          ? (input.endings as MovieTemplate['endings'])
          : {},
      characters,
      provenance,
    };

    return normalized;
  };

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
            const parsed = JSON.parse(String(v)) as unknown;
            return normalizeTemplate(parsed);
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

    try {
      const raw = localStorage.getItem('mg_active_game_data');
      if (
        !raw ||
        raw === 'null' ||
        raw === 'undefined' ||
        raw === '[object Object]'
      ) {
        return;
      }

      const parsed = JSON.parse(raw) as unknown;
      const normalized = normalizeTemplate(parsed);
      if (!normalized) return;

      const obj = isRecord(parsed) ? parsed : {};
      const metaOk = isRecord(obj.meta);
      const charactersOk = isRecord(obj.characters);
      const provenanceOk = isRecord(obj.provenance);
      const baseOk =
        typeof obj.projectId === 'string' &&
        typeof obj.version === 'string' &&
        typeof obj.owner === 'string';

      if (!metaOk || !charactersOk || !provenanceOk || !baseOk) {
        localStorage.setItem('mg_active_game_data', JSON.stringify(normalized));
        gameData.value = normalized;
      }
    } catch {
      // ignore
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

    const entry = String(sessionStorage.getItem('mg_play_entry') || '').trim();
    if (entry === 'shared') {
      const sharedId =
        String(sessionStorage.getItem('mg_shared_play_id') || '').trim() ||
        String(gameData.value?.requestId || '').trim();

      if (sharedId) {
        router.push(`/play/${encodeURIComponent(sharedId)}`);
        return;
      }

      router.push('/');
      return;
    }

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
