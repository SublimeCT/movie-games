import { ref, nextTick, onMounted, watch } from 'vue';
import { useRouter } from 'vue-router';
import type { Story } from '../types/Story';
import type { Ending, MovieTemplate } from '../types/movie';
import { db, type GameState } from '../utils/db';

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

const isRecord = (v: unknown): v is Record<string, unknown> =>
  Boolean(v) && typeof v === 'object';

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

const createId = () => {
  try {
    return crypto.randomUUID();
  } catch {
    return `p_${Date.now()}_${Math.random().toString(16).slice(2)}`;
  }
};

// Global state refs
const gameData = ref<Story | MovieTemplate | null>(null);
const endingData = ref<Ending | null>(null);

/**
 * 游戏状态管理 Hook
 * 使用 IndexedDB 持久化状态
 */
export function useGameState() {
  const router = useRouter();

  // Initialize from DB on mount
  onMounted(async () => {
    try {
      // Load active session
      const session = await db.getActiveSession();
      if (session) {
        // We need to load the full game template.
        // The session stores state, but maybe not the full template if we separate them?
        // In db.ts, GameState has: gameId, currentNodeId, playerState, historyStack, ending.
        // It does NOT have the template.
        // We need to fetch the template from played_games using gameId.
        
        // Wait, if we are in Designer, we might be editing a Draft.
        // But useGameState is mostly for "Play" mode or "Shared Data".
        // Let's check db.ts definition of GameState.
        
        // Refetching template:
        const template = await db.getPlayedGame(session.gameId);
        if (template) {
           gameData.value = template;
           // We might need to restore other session state here if needed by components,
           // but components usually ask for it. 
           // However, gameData is the static template.
           // If session has ending, we restore it.
           if (session.ending) {
             endingData.value = session.ending;
           }
        }
      }
    } catch (e) {
      console.error('Failed to restore game state:', e);
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
    c: Story['characters'][string],
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
   * @deprecated 应该直接使用 db.saveDraft
   */
  const persistHomeInputsFromTemplate = async (template: Story | MovieTemplate) => {
    // We can still use this to populate the Draft in DB if we want "Import" to overwrite Draft.
    // For now, let's keep the logic but maybe write to Draft DB?
    // The user requirement: "正在编辑的数据也保存到 indexeddb 中(新的表), 只能有一条数据"
    // So if we import to Design, we should overwrite Draft.
    
    // But this function was used for "Home Inputs" (localStorage).
    // I'll leave it for now but we might need to change Home.vue to read from Draft DB.
  };

  const loadGameData = async (
    data: Story | MovieTemplate,
    entry: 'owner' | 'import' | 'shared' = 'owner',
    path = '/game',
  ) => {
    sessionStorage.setItem('mg_play_entry', entry);
    sessionStorage.removeItem('mg_shared_play_id');

    // Clean up old local storage (migration)
    localStorage.removeItem('mg_current_node');
    localStorage.removeItem('mg_player_state');
    localStorage.removeItem('mg_history_stack');
    localStorage.removeItem('mg_active_game_data');

    endingData.value = null;

    // Normalize ID
    const _localId = data.requestId || (data as any).projectId || (data as any).id || createId();
    // Ensure data has this ID (if it was missing)
    if (!data.requestId && !(data as any).projectId) {
        (data as any).projectId = _localId;
    }

    // 1. Save to Played Games
    // Note: If we are entering "Design", we might also want to save to Draft?
    // If path is /design, we should save to Draft.
    if (path.includes('design')) {
        // Save to Draft
        await db.saveDraft(data as MovieTemplate); // Casting, assuming Story is compatible or handled
    } else {
        // Save to Played Games
        await db.savePlayedGame(data, entry);
        
        // 2. Initialize Active Session
        await db.setActiveSession({
            gameId: _localId,
            currentNodeId: 'start', // Default, Game.vue will handle actual start
            playerState: { flags: {}, variables: {} },
            historyStack: [],
        });
    }

    gameData.value = data;
    await nextTick();

    router.push(path);
  };

  /**
   * 开始新游戏
   */
  const handleGameStart = async (data: Story | MovieTemplate) => {
    await loadGameData(data, 'owner', '/game');
  };

  /**
   * 游戏结束
   */
  const handleGameEnd = async (ending: Ending) => {
    endingData.value = ending;
    
    // Update active session with ending
    const session = await db.getActiveSession();
    if (session) {
        session.ending = ending;
        await db.setActiveSession(session);
    }
    
    router.push('/ending');
  };

  /**
   * 清除当前游戏数据
   */
  const clearGameData = async () => {
    gameData.value = null;
    endingData.value = null;
    await db.clearActiveSession();
  };

  /**
   * 重新开始当前游戏
   */
  const handleRestartPlay = async () => {
    endingData.value = null;
    
    // Reset active session state but keep gameId
    const session = await db.getActiveSession();
    if (session) {
        session.currentNodeId = 'start'; // Or undefined to let Game.vue init
        session.playerState = { flags: {}, variables: {} };
        session.historyStack = [];
        session.ending = undefined;
        await db.setActiveSession(session);
    }

    const sharedId = String(
      sessionStorage.getItem('mg_shared_play_id') || '',
    ).trim();
    if (sharedId) {
      router.push(`/play/${encodeURIComponent(sharedId)}`);
      return;
    }

    const entry = String(sessionStorage.getItem('mg_play_entry') || '').trim();
    if (entry === 'shared') {
      const storedSharedId = String(gameData.value?.requestId || '').trim();

      if (storedSharedId) {
        router.push(`/play/${encodeURIComponent(storedSharedId)}`);
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
  const handleRemake = async () => {
    gameData.value = null;
    endingData.value = null;
    await db.clearActiveSession();
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
    clearGameData,
  };
}
