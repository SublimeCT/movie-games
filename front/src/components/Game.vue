<script setup lang="ts">
import { useStorage } from '@vueuse/core';
import { ArrowLeft, ChevronRight, Home as HomeIcon, X } from 'lucide-vue-next';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { useGameState } from '../hooks/useGameState';
import type { Character, Choice, Ending } from '../types/movie';
import CharacterAvatar from './ui/CharacterAvatar.vue';
import ThreeDCard from './ui/ThreeDCard.vue';

// 为选项按钮生成动画延迟
const getStaggerDelay = (index: number) => `${index * 50}ms`;

// 预计算粒子位置
const particles = Array.from({ length: 30 }, (_, i) => ({
  id: i,
  x: Math.random() * 100,
  y: Math.random() * 100,
  size: 2 + Math.random() * 4,
  duration: 10 + Math.random() * 14,
  delay: i * 0.6,
}));

const router = useRouter();

// 使用 hook 获取游戏数据和方法
const { gameData, handleGameEnd } = useGameState();

const baseTitle = document.title;
let titleApplied = false;

/**
 * 将当前剧情名追加到页面标题，离开页面时会恢复。
 */
const applyTitlePrefix = (storyTitleRaw: string) => {
  const storyTitle = String(storyTitleRaw || '').trim();
  if (!storyTitle) return;
  if (document.title.startsWith(`${storyTitle} - `)) {
    titleApplied = true;
    return;
  }
  document.title = `${storyTitle} - ${baseTitle}`;
  titleApplied = true;
};

/**
 * 恢复进入游戏页前的页面标题。
 */
const restoreTitle = () => {
  if (!titleApplied) return;
  document.title = baseTitle;
  titleApplied = false;
};

watch(
  () => String(gameData.value?.title || ''),
  (t) => {
    const title = String(t || '').trim();
    if (!title) return;
    applyTitlePrefix(title);
  },
  { immediate: true },
);

onUnmounted(() => {
  restoreTitle();
});

/**
 * Helper to find the start node ID from the data.
 * Checks for 'start', 'root', '1' or defaults to the first key.
 */
const startNodeId = computed(() => {
  const data = gameData.value;
  if (!data?.nodes) return '';
  const keys = Object.keys(data.nodes);
  if (keys.length === 0) return '';
  if (data.nodes.start) return 'start';
  if (keys.includes('start')) return 'start';
  if (keys.includes('root')) return 'root';
  if (keys.includes('1')) return '1';
  return keys[0];
});

const navigationError = ref<string>('');

const spotlightCardEl = ref<HTMLElement | null>(null);

const updateSpotlight = (e: PointerEvent, active: boolean) => {
  const el = spotlightCardEl.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;
  el.style.setProperty('--mg-spotlight-x', `${x}px`);
  el.style.setProperty('--mg-spotlight-y', `${y}px`);
  el.style.setProperty('--mg-spotlight-o', active ? '1' : '0');
  el.style.setProperty(
    '--mg-spotlight-color',
    active ? 'rgba(168, 85, 247, 0.25)' : 'rgba(255, 255, 255, 0.1)',
  );
};

const handleSpotlightMove = (e: PointerEvent) => updateSpotlight(e, true);
const handleSpotlightEnter = (e: PointerEvent) => updateSpotlight(e, true);
/**
 * Handles mouse leave event for the spotlight card.
 * Fades out the spotlight effect.
 */
const handleSpotlightLeave = () => {
  const el = spotlightCardEl.value;
  if (!el) return;
  el.style.setProperty('--mg-spotlight-o', '0');
};

/**
 * Persistent state for the current node ID.
 */
const currentNodeId = useStorage<string>('mg_current_node', '');

/**
 * Persistent state for player variables and flags.
 */
const playerState = useStorage('mg_player_state', { flags: {}, variables: {} });

const affinityState = useStorage<Record<string, number>>(
  'mg_affinity_state',
  {},
);

/**
 * History stack for the back button functionality.
 * Stores previous node IDs and player states.
 */
// biome-ignore lint/suspicious/noExplicitAny: State needs to be flexible
const historyStack = useStorage<{ nodeId: string; state: any }[]>(
  'mg_history_stack',
  [],
);

/**
 * Watch for node changes to check if the new node is an ending.
 */
watch(currentNodeId, (newId) => {
  if (newId) checkEnding(newId);
});

/**
 * Checks if the given node ID corresponds to an ending.
 * If so, emits the 'end' event.
 * @param {string} nodeId - The node ID to check.
 */
const checkEnding = (nodeId: string) => {
  const data = gameData.value;
  if (!data) return;

  const node = data.nodes?.[nodeId];

  // Check for explicit endings
  if (data.endings?.[nodeId]) {
    handleGameEnd({
      ...data.endings[nodeId],
      endingKey: nodeId,
      nodeId,
      reachedAt: new Date().toISOString(),
    });
    return;
  }

  const choices = node?.choices || [];
  if (node && choices.length === 0) {
    // Special handling for 'start' node with no choices:
    // Instead of triggering ending, find the first node with choices
    if (nodeId === 'start' || nodeId === 'root') {
      const keys = Object.keys(data.nodes);
      for (const key of keys) {
        if (data.nodes[key]?.choices?.length) {
          currentNodeId.value = key;
          return;
        }
      }
    }

    // If no alternative node found, trigger ending
    handleGameEnd({
      type: 'neutral',
      description:
        typeof node?.content === 'string'
          ? node.content
          : // biome-ignore lint/suspicious/noExplicitAny: Handle legacy object format
            (node?.content as any)?.text || '故事结束',
      nodeId,
      reachedAt: new Date().toISOString(),
    });
    return;
  }
};

/**
 * Resets navigation state to the start node.
 */
const resetToStart = () => {
  const start = startNodeId.value;
  if (!start) return;
  currentNodeId.value = start;
  historyStack.value = [];
  navigationError.value = '';
};

/**
 * Navigates to home and clears current run state.
 */
const goHome = () => {
  localStorage.removeItem('mg_current_node');
  localStorage.removeItem('mg_player_state');
  localStorage.removeItem('mg_history_stack');
  localStorage.removeItem('mg_ending');
  localStorage.removeItem('mg_affinity_state');
  localStorage.removeItem('mg_active_game_data');
  gameData.value = null;
  navigationError.value = '';
  router.push('/');
};

const confirmGoHomeOpen = ref(false);

const openConfirmGoHome = () => {
  if (!gameData.value) {
    goHome();
    return;
  }
  confirmGoHomeOpen.value = true;
};

const closeConfirmGoHome = () => {
  confirmGoHomeOpen.value = false;
};

const confirmGoHome = () => {
  confirmGoHomeOpen.value = false;
  goHome();
};

/**
 * Initialize the game state on mount.
 * Sets the start node if not set, and checks for endings.
 */
onMounted(() => {
  // Try to load data from sessionStorage first (from Generating page)
  // This is deprecated as we now use global state in App.vue
  /*
  const storedData = sessionStorage.getItem('mg_game_data');
  if (storedData) {
    try {
      localData.value = JSON.parse(storedData);
      sessionStorage.removeItem('mg_game_data');
    } catch {
      // Ignore parse errors
    }
  }
  */

  const data = gameData.value;
  if (!data?.nodes) return;
  if (
    !currentNodeId.value ||
    (!data.nodes[currentNodeId.value] && !data.endings?.[currentNodeId.value])
  ) {
    resetToStart();
  }
  checkEnding(currentNodeId.value);
});

watch(
  () => gameData.value,
  (next) => {
    if (!next?.nodes) return;
    if (
      !currentNodeId.value ||
      (!next.nodes[currentNodeId.value] && !next.endings?.[currentNodeId.value])
    ) {
      resetToStart();
    }
  },
  { immediate: true },
);

/**
 * Handles the back button click.
 * Pops the last state from history and restores it.
 */
const handleBack = () => {
  if (historyStack.value.length === 0) return;
  const last = historyStack.value.pop();
  if (last) {
    currentNodeId.value = last.nodeId;
    playerState.value = last.state;
    navigationError.value = '';
  }
};

/**
 * Computed property for the current node object.
 */
const currentNode = computed(
  () => gameData.value?.nodes?.[currentNodeId.value],
);

const missingNode = computed(() => {
  const data = gameData.value;
  if (!data?.nodes) return false;
  if (!currentNodeId.value) return false;
  if (data.endings?.[currentNodeId.value]) return false;
  return !data.nodes[currentNodeId.value];
});

/**
 * Computed property for the characters present in the current node.
 * Infers characters from the node data or defaults to the protagonist.
 */
const currentAgents = computed(() => {
  const data = gameData.value;
  let agents: Character[] = [];

  // 1. Try to find explicitly listed characters
  if (
    currentNode.value?.characters &&
    currentNode.value.characters.length > 0
  ) {
    currentNode.value.characters.forEach((name, idx) => {
      const n = (name || '').trim();
      if (!n) return;
      const char = Object.values(data?.characters || {}).find(
        (c) => c.name === n || c.id === n,
      );
      if (char) {
        agents.push(char);
        return;
      }

      agents.push({
        id: `mg_unknown_${idx}_${n}`,
        name: n,
        gender: '其他',
        age: 0,
        role: '',
        background: '',
      });
    });
  }

  if (agents.length === 0 && data?.characters) {
    const selected = selectDefaultCharacter(data.characters);
    if (selected) agents.push(selected);
  }

  const byId = new Set<string>();
  agents = agents.filter((a) => {
    if (!a?.id) return false;
    if (byId.has(a.id)) return false;
    byId.add(a.id);
    return true;
  });

  if (agents.length === 0) {
    const seed = (data?.projectId || data?.title || 'mg')
      .split('')
      .reduce((a, b) => a + b.charCodeAt(0), 0);
    const gender = seed % 2 === 0 ? 'Male' : 'Female';
    agents.push({
      id: 'mg_player',
      name: '我',
      gender,
      age: 0,
      role: '',
      background: '',
    });
  }

  return agents.slice(0, 3);
});

/**
 * Normalizes the gender string for avatar display.
 * @param {string | undefined} raw - The raw gender string.
 * @returns {string} The normalized gender ('Male', 'Female', or 'Other').
 */
const normalizeGenderTag = (raw?: string) => {
  const g = (raw || '').trim().toLowerCase();

  if (/(female|woman|girl|\bf\b|女|女性|女生|女士|女人)/.test(g))
    return 'Female';
  if (/(male|man|boy|\bm\b|男|男性|男生|先生|男人)/.test(g)) return 'Male';
  return 'Other';
};

/**
 * @param {Record<string, Character>} characters
 * @returns {Character | null}
 */
const selectDefaultCharacter = (characters: Record<string, Character>) => {
  const entries = Object.entries(characters);
  if (entries.length === 0) return null;

  const scored = entries
    .map(([key, c]) => {
      const name = (c.name || '').toLowerCase();
      const role = (c.role || '').toLowerCase();
      let score = 0;

      if (/player|protagonist|main/.test(key.toLowerCase())) score += 5;
      if (name.includes('主角') || name === '我') score += 6;
      if (role.includes('主角') || role.includes('protagonist')) score += 3;
      if (c.age && c.age > 0) score += 1;

      return { score, c };
    })
    .sort((a, b) => b.score - a.score);

  return scored[0]?.c ?? null;
};

const protagonistName = computed(() => {
  const data = gameData.value;
  const selected = data?.characters
    ? selectDefaultCharacter(data.characters)
    : null;
  return String(selected?.name || '').trim();
});

watch(
  () => gameData.value,
  (next) => {
    const chars = next?.characters ?? {};
    const protagonist = protagonistName.value;
    const names = Object.values(chars)
      .map((c) => String(c.name || '').trim())
      .filter(Boolean);

    const base: Record<string, number> = {};
    for (const name of names) {
      if (protagonist && name === protagonist) continue;
      const cur = affinityState.value[name];
      const curNum = typeof cur === 'number' && Number.isFinite(cur) ? cur : 50;
      base[name] = Math.max(0, Math.min(100, Math.round(curNum)));
    }

    affinityState.value = base;
  },
  { immediate: true },
);

/**
 * Computed property for the available choices in the current node.
 */
const availableChoices = computed(() => {
  if (!currentNode.value?.choices) return [];
  return currentNode.value.choices;
});

/**
 * Handles a choice selection.
 * Updates history and navigates to the next node.
 * @param {Choice} choice - The selected choice.
 */
const handleChoice = async (choice: Choice) => {
  navigationError.value = '';

  const canonicalizeEndingId = (raw: string) => {
    const v = (raw || '').trim();
    if (!v) return v;
    if (v === 'bad_end' || v === 'end_bad' || v === 'bad') return 'ending_bad';
    if (v === 'good_end' || v === 'end_good' || v === 'good')
      return 'ending_good';
    if (v === 'neutral_end' || v === 'end_neutral' || v === 'neutral')
      return 'ending_neutral';
    return v;
  };

  const nextId = canonicalizeEndingId(choice.nextNodeId);

  const data = gameData.value;
  const effect = choice.affinityEffect;
  if (data?.characters && effect) {
    const idToName = new Map(
      Object.values(data.characters)
        .map(
          (c) =>
            [String(c.id || '').trim(), String(c.name || '').trim()] as const,
        )
        .filter(([id, name]) => Boolean(id && name)),
    );

    const present = Array.isArray(currentNode.value?.characters)
      ? (currentNode.value?.characters as string[])
      : [];

    const allowed = new Set(
      present
        .map((raw) => {
          const v = String(raw || '').trim();
          return idToName.get(v) || v;
        })
        .filter(Boolean),
    );

    const rawTarget = String(effect.characterId || '').trim();
    const target = idToName.get(rawTarget) || rawTarget;
    const protagonist = protagonistName.value;

    if (
      target &&
      allowed.has(target) &&
      (!protagonist || target !== protagonist)
    ) {
      const cur = affinityState.value[target];
      const curNum = typeof cur === 'number' && Number.isFinite(cur) ? cur : 50;
      const delta = Math.max(
        -20,
        Math.min(20, Math.round(Number(effect.delta) || 0)),
      );
      const next = Math.max(0, Math.min(100, Math.round(curNum + delta)));
      affinityState.value = { ...affinityState.value, [target]: next };
    }
  }

  // Navigate
  if (nextId === 'END') {
    // Try to construct a default ending if not present
    const ending: Ending = {
      description: '故事结束',
      type: 'neutral',
      nodeId: currentNodeId.value,
      reachedAt: new Date().toISOString(),
    };
    handleGameEnd(ending);
    return;
  }

  // Check if nextNodeId is an ending ID
  if (data?.endings?.[nextId]) {
    handleGameEnd({
      ...data.endings[nextId],
      endingKey: nextId,
      nodeId: currentNodeId.value,
      reachedAt: new Date().toISOString(),
    });
    return;
  }

  if (!data?.nodes?.[nextId]) {
    navigationError.value = `无效跳转：${choice.nextNodeId}`;
    return;
  }

  historyStack.value.push({
    nodeId: currentNodeId.value,
    state: JSON.parse(JSON.stringify(playerState.value)),
  });

  currentNodeId.value = nextId;
};

/**
 * Determines the emotion of a character based on the current text or a hash fallback.
 * @param {Character} agent - The character to determine emotion for.
 * @returns {string} The emotion string.
 */
const getEmotion = (agent: Character) => {
  const text = (currentNode.value?.content || '').toLowerCase();
  if (text.match(/angry|furious|mad|rage|shout/)) return 'angry';
  if (text.match(/sad|cry|weep|tear|grief|depress/)) return 'sad';
  if (text.match(/happy|smile|laugh|joy|delight/)) return 'happy';
  if (text.match(/surprise|shock|gasp|stun/)) return 'surprised';

  const name = String(agent.name || '').trim();
  const protagonist = protagonistName.value;
  if (name && (!protagonist || name !== protagonist)) {
    const cur = affinityState.value[name];
    const curNum = typeof cur === 'number' && Number.isFinite(cur) ? cur : 50;
    const v = Math.max(0, Math.min(100, curNum));
    if (v >= 75) return 'happy';
    if (v <= 25) return 'angry';
    if (v <= 45) return 'sad';
    return 'neutral';
  }

  const emotions = ['neutral', 'happy', 'sad', 'angry', 'surprised'];
  const hash = (currentNodeId.value + agent.name)
    .split('')
    .reduce((a, b) => a + b.charCodeAt(0), 0);
  return emotions[hash % emotions.length];
};

const backgroundImageBase64 = computed(() =>
  (gameData.value?.backgroundImageBase64 || '').trim(),
);

const backgroundBaseStyle = computed<Record<string, string>>(() => {
  const img = backgroundImageBase64.value;
  if (img) {
    return {
      backgroundImage: `url(${img})`,
      backgroundSize: 'cover',
      backgroundPosition: 'center',
      backgroundRepeat: 'no-repeat',
      transform: 'scale(1.02)',
      filter: 'saturate(0.9) contrast(1.05) brightness(0.72)',
    };
  }

  return {
    backgroundImage:
      'radial-gradient(1200px circle at 20% 20%, rgba(168,85,247,0.18), transparent 55%), radial-gradient(900px circle at 80% 30%, rgba(59,130,246,0.12), transparent 55%), radial-gradient(900px circle at 50% 90%, rgba(236,72,153,0.10), transparent 60%), linear-gradient(180deg, #05060a 0%, #000 60%, #000 100%)',
    backgroundSize: 'cover',
    backgroundPosition: 'center',
    backgroundRepeat: 'no-repeat',
    transform: 'none',
    filter: 'none',
  };
});

const backgroundMaskStyle = computed<Record<string, string>>(() => ({
  backgroundImage:
    'linear-gradient(180deg, rgba(0,0,0,0.72) 0%, rgba(0,0,0,0.45) 45%, rgba(0,0,0,0.82) 100%), radial-gradient(1200px circle at 20% 20%, rgba(168,85,247,0.16), transparent 55%), radial-gradient(900px circle at 80% 30%, rgba(59,130,246,0.10), transparent 55%), radial-gradient(900px circle at 50% 90%, rgba(236,72,153,0.08), transparent 60%)',
}));

/**
 * Currently selected agent for the mobile popup.
 */
const selectedAgent = ref<Character | null>(null);

/**
 * Opens the character details popup on mobile.
 * @param {Character} agent - The character to display.
 */
const handleAvatarClick = (agent: Character) => {
  selectedAgent.value = agent;
};
</script>

<template>
  <!-- Mobile Avatar Popup -->
  <Transition 
    enter-active-class="transition duration-300 ease-out"
    enter-from-class="opacity-0 scale-95"
    enter-to-class="opacity-100 scale-100"
    leave-active-class="transition duration-200 ease-in"
    leave-from-class="opacity-100 scale-100"
    leave-to-class="opacity-0 scale-95"
  >
    <div v-if="selectedAgent" class="fixed inset-0 z-[100] flex items-center justify-center p-4">
      <div class="absolute inset-0 bg-black/80 backdrop-blur-md" @click="selectedAgent = null"></div>
      <div class="relative w-full max-w-sm bg-black/80 backdrop-blur-3xl border border-white/10 rounded-3xl overflow-hidden shadow-[0_0_50px_rgba(168,85,247,0.2)] p-8 flex flex-col items-center gap-6 ring-1 ring-white/5">
        <!-- Decoration -->
        <div class="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-transparent via-purple-500 to-transparent"></div>
        <div class="absolute -inset-1 bg-gradient-to-r from-purple-500/20 via-transparent to-pink-500/20 blur-xl opacity-50 pointer-events-none"></div>

        <button @click="selectedAgent = null" class="absolute top-4 right-4 text-white/50 hover:text-white p-2 rounded-full hover:bg-white/10 transition-colors z-20">
          <X class="w-6 h-6" />
        </button>
        
        <div class="relative z-10">
            <div class="absolute inset-0 bg-purple-500/30 blur-2xl rounded-full scale-110"></div>
            <CharacterAvatar 
            :name="selectedAgent.name" 
            :gender="normalizeGenderTag(selectedAgent.gender)" 
            :emotion="getEmotion(selectedAgent)" 
            :avatarPath="selectedAgent.avatarPath"
            className="w-32 h-32 shadow-2xl relative z-10" 
            />
        </div>
        
        <div class="text-center space-y-3 relative z-10 w-full">
          <h3 class="text-3xl font-bold text-white tracking-tight drop-shadow-[0_2px_10px_rgba(0,0,0,0.5)]">{{ selectedAgent.name }}</h3>
          <div class="flex items-center justify-center gap-2 text-xs text-purple-300 uppercase tracking-[0.2em] font-bold border-y border-white/10 py-2 w-full">
            <span>{{ selectedAgent.gender }}</span>
            <span v-if="selectedAgent.age">· {{ selectedAgent.age }}岁</span>
            <span v-if="selectedAgent.role">· {{ selectedAgent.role }}</span>
          </div>
          <p class="text-neutral-300 text-sm leading-relaxed line-clamp-6 font-light">{{ selectedAgent.background || '暂无描述' }}</p>
        </div>
      </div>
    </div>
  </Transition>

  <Transition enter-active-class="animate-in fade-in duration-200" leave-active-class="animate-out fade-out duration-150">
    <div v-if="confirmGoHomeOpen" class="fixed inset-0 z-[110] flex items-center justify-center p-4">
      <div class="absolute inset-0 bg-black/80 backdrop-blur-md" @click="closeConfirmGoHome"></div>
      <div class="w-full max-w-md bg-neutral-900/90 border border-white/10 rounded-2xl overflow-hidden shadow-2xl relative z-10">
        <div class="px-5 py-4 flex items-center justify-between border-b border-white/10">
          <div class="font-bold text-white">返回首页确认</div>
          <button @click="closeConfirmGoHome" class="p-2 rounded-lg hover:bg-white/5 transition-colors">
            <X class="w-5 h-5 text-white/70" />
          </button>
        </div>

        <div class="p-5 text-sm text-white/70 leading-relaxed whitespace-pre-line">
          返回首页将会清空当前游戏进度与当前剧情。
        </div>

        <div class="px-5 py-4 bg-black/20 border-t border-white/10 flex items-center justify-end gap-3">
          <button @click="closeConfirmGoHome" class="px-5 py-2 rounded-xl bg-white/10 hover:bg-white/20 text-white/80 font-medium transition-colors">
            取消
          </button>
          <button @click="confirmGoHome" class="px-5 py-2 rounded-xl font-bold transition-colors bg-red-500/20 hover:bg-red-500/30 text-red-100 border border-red-500/20">
            确定返回
          </button>
        </div>
      </div>
    </div>
  </Transition>

  <div class="h-screen w-full bg-black text-white overflow-hidden relative font-sans">
      <!-- Background Layer -->
      <div class="absolute inset-0 z-0 bg-black" :style="backgroundBaseStyle"></div>
      <div class="absolute inset-0 z-0 pointer-events-none" :style="backgroundMaskStyle"></div>

      <!-- 动态背景动画 - 仅在无背景图时显示 -->
      <Transition
        enter-active-class="transition-opacity duration-1000"
        enter-from-class="opacity-0"
        enter-to-class="opacity-100"
        leave-active-class="transition-opacity duration-500"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <div v-if="!backgroundImageBase64" class="absolute inset-0 z-0 pointer-events-none overflow-hidden">
          <!-- 流动的极光效果 -->
          <div class="aurora-blob aurora-blob-1"></div>
          <div class="aurora-blob aurora-blob-2"></div>
          <div class="aurora-blob aurora-blob-3"></div>
          <div class="aurora-blob aurora-blob-4"></div>

          <!-- 漂浮的光点粒子 -->
          <div class="absolute inset-0">
            <div v-for="p in particles" :key="p.id"
                 class="floating-particle"
                 :style="{
                   '--delay': `${p.delay}s`,
                   '--x': `${p.x}%`,
                   '--y': `${p.y}%`,
                   '--size': `${p.size}px`,
                   '--duration': `${p.duration}s`
                 }">
            </div>
          </div>

          <!-- 动态光束 -->
          <div class="light-beam light-beam-1"></div>
          <div class="light-beam light-beam-2"></div>

          <!-- 动态网格线 -->
          <div class="absolute inset-0 grid-lines"></div>
        </div>
      </Transition>

      <!-- 胶片颗粒效果 -->
      <div class="absolute inset-0 z-0 opacity-[0.14] mix-blend-overlay pointer-events-none animate-[grain_8s_steps(10)_infinite] bg-[url('data:image/svg+xml,%3Csvg%20xmlns=%22http://www.w3.org/2000/svg%22%20width=%22300%22%20height=%22300%22%3E%3Cfilter%20id=%22n%22%3E%3CfeTurbulence%20type=%22fractalNoise%22%20baseFrequency=%220.8%22%20numOctaves=%223%22%20stitchTiles=%22stitch%22/%3E%3C/filter%3E%3Crect%20width=%22300%22%20height=%22300%22%20filter=%22url(%23n)%22%20opacity=%220.45%22/%3E%3C/svg%3E')]"></div>

      <!-- Top Bar -->
      <header class="absolute top-0 left-0 w-full p-6 flex justify-between items-center z-50 pointer-events-none">
          <div class="pointer-events-auto">
              <h2 class="text-white/50 text-xs tracking-[0.2em] uppercase font-bold backdrop-blur-sm px-3 py-1 rounded-full border border-white/10">{{ gameData?.title || 'Movie Game' }}</h2>
          </div>
          
          <div class="flex items-center gap-3 pointer-events-auto">
            <button v-if="historyStack.length > 0" @click="handleBack" class="bg-black/40 backdrop-blur-md border border-white/10 hover:border-purple-500/50 text-white/70 hover:text-white px-4 py-2 rounded-lg transition-all text-sm flex items-center gap-2 group">
              <ArrowLeft class="w-4 h-4" />
              <span class="hidden md:inline">返回上一步</span>
            </button>

            <button @click="openConfirmGoHome" class="relative bg-black/40 backdrop-blur-md border border-white/10 hover:border-purple-500/50 text-white/80 hover:text-white px-5 py-2.5 rounded-xl transition-all text-sm flex items-center gap-2 group overflow-hidden shadow-[0_0_10px_rgba(0,0,0,0.5)] hover:shadow-[0_0_20px_rgba(168,85,247,0.4)]">
              <span class="absolute inset-0 bg-gradient-to-r from-purple-600/10 via-pink-600/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300"></span>
              <span class="absolute bottom-0 left-0 w-full h-[1px] bg-gradient-to-r from-transparent via-purple-500 to-transparent scale-x-0 group-hover:scale-x-100 transition-transform duration-500"></span>
              <span class="relative flex items-center gap-2 font-bold tracking-wide">
                <HomeIcon class="w-4 h-4 group-hover:scale-110 group-hover:-translate-x-0.5 transition-transform" />
                <span class="hidden md:inline">返回首页</span>
              </span>
            </button>
          </div>
      </header>
  
      <!-- Main Stage -->
      <main class="relative z-10 h-full flex flex-col items-center justify-end md:justify-center pb-0 md:pb-20">
          
          <!-- Characters Stage -->
          <div class="flex-1 w-full flex items-center justify-center relative z-30">
              <TransitionGroup 
                tag="div"
                class="flex items-end justify-center w-full"
                enter-active-class="transition-[transform,opacity] duration-700 ease-out"
                enter-from-class="opacity-0 translate-y-10 scale-95"
                enter-to-class="opacity-100 translate-y-0 scale-100"
                leave-active-class="transition-[transform,opacity] duration-500 ease-in"
                leave-from-class="opacity-100 scale-100"
                leave-to-class="opacity-0 scale-90"
              >
                  <div 
                    v-for="agent in currentAgents" 
                    :key="agent.id" 
                    class="flex flex-col items-center mx-4 md:mx-12 group relative cursor-pointer"
                    @click="handleAvatarClick(agent)"
                  >
                       <div class="relative">
                           <div class="absolute inset-0 bg-purple-500/20 blur-xl rounded-full opacity-0 group-hover:opacity-100 transition-opacity duration-500"></div>
                           <CharacterAvatar 
                                :name="agent.name" 
                                :gender="normalizeGenderTag(agent.gender)" 
                                :emotion="getEmotion(agent)" 
                                :avatarPath="agent.avatarPath"
                                className="w-32 h-32 md:w-48 md:h-48 drop-shadow-2xl relative z-10 transform transition-transform duration-500 hover:scale-110" 
                           />
                       </div>
                       <div class="mt-4 px-4 py-1 bg-black/60 backdrop-blur-md border border-white/10 rounded-full">
                           <span class="text-xs md:text-sm font-bold tracking-widest text-white/90 uppercase">{{ agent.name }}</span>
                       </div>

                       <div class="absolute top-0 left-1/2 -translate-x-1/2 translate-y-[calc(-100%-12px)] opacity-0 group-hover:opacity-100 pointer-events-none transition-opacity duration-200">
                          <div class="min-w-[240px] max-w-[280px] bg-black/90 backdrop-blur-xl border border-white/10 rounded-2xl px-5 py-4 shadow-2xl z-50">
                            <div class="text-base font-bold text-white mb-2">{{ agent.name }}</div>
                            
                            <div class="space-y-1.5 mb-3">
                                <div class="flex items-center gap-2 text-xs text-white/60">
                                    <span class="px-1.5 py-0.5 rounded bg-white/10 border border-white/5">{{ agent.gender || '未知' }}</span>
                                    <span v-if="agent.age" class="px-1.5 py-0.5 rounded bg-white/10 border border-white/5">{{ agent.age }}岁</span>
                                </div>
                                <div v-if="agent.role" class="text-xs text-purple-300 font-medium">
                                    {{ agent.role }}
                                </div>
                            </div>

                            <div class="text-xs text-white/70 leading-relaxed border-t border-white/10 pt-2 line-clamp-5">
                                {{ agent.background || '暂无描述' }}
                            </div>
                          </div>
                       </div>
                  </div>
              </TransitionGroup>
          </div>
          
          <!-- Dialogue & Choice Box -->
          <div class="w-full max-w-4xl px-4 pb-8 md:pb-0 z-20">
            <ThreeDCard>
              <div
                ref="spotlightCardEl"
                class="mg-spotlight relative overflow-hidden rounded-3xl border border-white/10 bg-black/60 backdrop-blur-3xl shadow-[0_0_50px_rgba(0,0,0,0.5)] ring-1 ring-white/5 p-8 md:p-12 transition-all duration-500 hover:shadow-[0_0_80px_rgba(168,85,247,0.15)] hover:border-white/20"
                @pointermove="handleSpotlightMove"
                @pointerenter="handleSpotlightEnter"
                @pointerleave="handleSpotlightLeave"
              >
                  <!-- Spotlight effect layer -->
                  <div 
                    class="pointer-events-none absolute -inset-px opacity-0 transition duration-300"
                    style="opacity: var(--mg-spotlight-o); background: radial-gradient(600px circle at var(--mg-spotlight-x) var(--mg-spotlight-y), var(--mg-spotlight-color), transparent 40%);"
                  ></div>
                  
                  <!-- Decoration -->
                  <div class="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-transparent via-purple-500/50 to-transparent"></div>

                  <!-- Narrative Text -->
                  <div class="min-h-[100px] mb-8 flex items-center justify-center">
                      <Transition 
                        mode="out-in"
                        enter-active-class="transition-all duration-500 ease-out delay-100"
                        enter-from-class="opacity-0 translate-y-2"
                        enter-to-class="opacity-100 translate-y-0"
                        leave-active-class="transition-all duration-200 ease-in"
                        leave-from-class="opacity-100"
                        leave-to-class="opacity-0 -translate-y-2"
                      >
                          <p :key="currentNodeId" class="text-lg md:text-2xl leading-relaxed font-light text-neutral-100 text-center drop-shadow-md">
                              {{ (typeof currentNode?.content === 'string' ? currentNode.content : (currentNode?.content as any)?.text) || (gameData ? '...' : '没有可用的游戏数据，请返回首页重新生成或导入。') }}
                          </p>
                      </Transition>
                  </div>

                  <div v-if="navigationError || missingNode" class="mb-6 text-sm text-red-200/90 bg-red-500/10 border border-red-500/20 rounded-xl px-4 py-3">
                    {{ navigationError || `节点不存在：${currentNodeId}` }}
                  </div>

                  <!-- Choices -->
                  <div class="flex flex-col gap-3">
                       <TransitionGroup
                         tag="div"
                         name="choice"
                       >
                         <button v-for="(choice, idx) in availableChoices"
                                 :key="`${currentNodeId}-${idx}`"
                                 @click="handleChoice(choice)"
                                 class="cinematic-choice group relative"
                                 :style="{ 'animation-delay': getStaggerDelay(idx), '--delay': getStaggerDelay(idx) }"
                         >
                             <!-- 胶片孔装饰 -->
                             <div class="film-perforation-left">
                               <span v-for="i in 5" :key="i" class="film-hole"></span>
                             </div>
                             <div class="film-perforation-right">
                               <span v-for="i in 5" :key="i" class="film-hole"></span>
                             </div>

                             <!-- 动态背景层 -->
                             <div class="choice-bg"></div>
                             <div class="choice-shine"></div>

                             <!-- 内容区域 -->
                             <div class="choice-content">
                               <div class="choice-text">
                                 <span class="text-gradient">{{ choice.text }}</span>
                               </div>

                               <!-- 交互图标 -->
                               <div class="choice-icon">
                                 <div class="icon-ring"></div>
                                 <ChevronRight class="chevron" :size="20" />
                               </div>
                             </div>

                             <!-- 悬停时的光效 -->
                             <div class="choice-glow"></div>
                         </button>
                       </TransitionGroup>

                       <template v-if="!gameData || ((navigationError || missingNode) && historyStack.length > 0)">
                         <Transition
                           enter-active-class="transition-all duration-500 ease-out"
                           enter-from-class="opacity-0 translate-y-4"
                           enter-to-class="opacity-100 translate-y-0"
                         >
                           <button v-if="!gameData" @click="openConfirmGoHome" class="w-full text-left bg-white/5 hover:bg-white/10 border border-white/10 hover:border-purple-500/50 p-4 md:p-5 rounded-xl transition-all duration-300">
                             <span class="text-neutral-200 text-base md:text-lg font-light tracking-wide">返回首页</span>
                           </button>

                           <button v-else-if="(navigationError || missingNode) && historyStack.length > 0" @click="handleBack" class="w-full text-left bg-white/5 hover:bg-white/10 border border-white/10 hover:border-purple-500/50 p-4 md:p-5 rounded-xl transition-all duration-300">
                             <span class="text-neutral-200 text-base md:text-lg font-light tracking-wide">回到上一步</span>
                           </button>
                         </Transition>
                       </template>
                  </div>
                  
                  <!-- Node ID Overlay -->
                   <div class="absolute bottom-1 right-2 text-[10px] text-white/30 font-mono select-none pointer-events-none">
                     {{ currentNodeId }}
                   </div>
              </div>
            </ThreeDCard>
          </div>
      </main>
  </div>
</template>

<style scoped>
.mg-spotlight {
  --mg-spotlight-x: 50%;
  --mg-spotlight-y: 50%;
  --mg-spotlight-o: 0;
}

.mg-spotlight::before {
  content: '';
  position: absolute;
  inset: -2px;
  background: radial-gradient(
    600px circle at var(--mg-spotlight-x) var(--mg-spotlight-y),
    rgba(168, 85, 247, 0.26),
    transparent 55%
  );
  opacity: var(--mg-spotlight-o);
  transition: opacity 160ms ease;
  pointer-events: none;
}

.mg-spotlight::after {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(90deg, rgba(168, 85, 247, 0.10), transparent 55%);
  opacity: 0.35;
  pointer-events: none;
}
</style>

<style scoped>
@keyframes grain {
  0% { transform: translate3d(0, 0, 0) }
  10% { transform: translate3d(-5%, -8%, 0) }
  20% { transform: translate3d(-10%, 6%, 0) }
  30% { transform: translate3d(7%, -10%, 0) }
  40% { transform: translate3d(-6%, 14%, 0) }
  50% { transform: translate3d(-12%, -12%, 0) }
  60% { transform: translate3d(12%, 2%, 0) }
  70% { transform: translate3d(2%, 12%, 0) }
  80% { transform: translate3d(-8%, 2%, 0) }
  90% { transform: translate3d(10%, -6%, 0) }
  100% { transform: translate3d(0, 0, 0) }
}

/* ==================== 动态背景动画 ==================== */

/* 极光效果 */
.aurora-blob {
  position: absolute;
  border-radius: 50%;
  filter: blur(60px);
  animation: aurora-float 25s ease-in-out infinite;
}

.aurora-blob-1 {
  width: 80vw;
  height: 80vw;
  background: radial-gradient(ellipse at center, rgba(139, 92, 246, 0.5) 0%, rgba(139, 92, 246, 0.1) 40%, transparent 70%);
  top: -30%;
  left: -20%;
  animation-delay: 0s;
}

.aurora-blob-2 {
  width: 70vw;
  height: 70vw;
  background: radial-gradient(ellipse at center, rgba(59, 130, 246, 0.5) 0%, rgba(59, 130, 246, 0.1) 40%, transparent 70%);
  top: 20%;
  right: -25%;
  animation-delay: -8s;
}

.aurora-blob-3 {
  width: 60vw;
  height: 60vw;
  background: radial-gradient(ellipse at center, rgba(236, 72, 153, 0.4) 0%, rgba(236, 72, 153, 0.1) 40%, transparent 70%);
  bottom: -20%;
  left: 20%;
  animation-delay: -16s;
}

.aurora-blob-4 {
  width: 50vw;
  height: 50vw;
  background: radial-gradient(ellipse at center, rgba(34, 211, 238, 0.3) 0%, rgba(34, 211, 238, 0.05) 40%, transparent 70%);
  bottom: 30%;
  right: 10%;
  animation-delay: -12s;
}

@keyframes aurora-float {
  0%, 100% {
    transform: translate(0, 0) rotate(0deg) scale(1);
  }
  25% {
    transform: translate(8%, 12%) rotate(15deg) scale(1.15);
  }
  50% {
    transform: translate(-6%, 8%) rotate(-10deg) scale(0.9);
  }
  75% {
    transform: translate(12%, -8%) rotate(8deg) scale(1.05);
  }
}

/* 漂浮粒子 */
.floating-particle {
  position: absolute;
  width: var(--size, 3px);
  height: var(--size, 3px);
  background: radial-gradient(circle, rgba(255, 255, 255, 1) 0%, rgba(168, 85, 247, 0.5) 50%, transparent 70%);
  border-radius: 50%;
  left: var(--x, 50%);
  top: var(--y, 50%);
  animation: particle-float var(--duration, 12s) ease-in-out infinite;
  animation-delay: var(--delay, 0s);
  box-shadow: 0 0 6px rgba(168, 85, 247, 0.6), 0 0 12px rgba(168, 85, 247, 0.3);
}

@keyframes particle-float {
  0% {
    transform: translate(0, 0) scale(1);
    opacity: 0;
  }
  15% {
    opacity: 1;
  }
  85% {
    opacity: 1;
  }
  100% {
    transform: translate(calc(var(--x) * 0.15 - 10%), -80%) scale(0.3);
    opacity: 0;
  }
}

/* 动态光束 */
.light-beam {
  position: absolute;
  width: 2px;
  height: 200%;
  top: -50%;
  background: linear-gradient(180deg,
    transparent 0%,
    rgba(168, 85, 247, 0.1) 20%,
    rgba(168, 85, 247, 0.3) 50%,
    rgba(168, 85, 247, 0.1) 80%,
    transparent 100%
  );
  filter: blur(2px);
  animation: beam-sweep 20s ease-in-out infinite;
}

.light-beam-1 {
  left: 30%;
  animation-delay: 0s;
}

.light-beam-2 {
  left: 70%;
  animation-delay: -10s;
}

@keyframes beam-sweep {
  0%, 100% {
    transform: translateX(-100px) rotate(-15deg);
    opacity: 0.3;
  }
  50% {
    transform: translateX(100px) rotate(15deg);
    opacity: 0.8;
  }
}

/* 动态网格线 */
.grid-lines {
  background-image:
    linear-gradient(rgba(168, 85, 247, 0.06) 1px, transparent 1px),
    linear-gradient(90deg, rgba(168, 85, 247, 0.06) 1px, transparent 1px);
  background-size: 60px 60px;
  animation: grid-move 40s linear infinite;
}

@keyframes grid-move {
  0% {
    background-position: 0 0;
  }
  100% {
    background-position: 60px 60px;
  }
}

/* ==================== 电影感选项按钮 ==================== */

.cinematic-choice {
  position: relative;
  width: 100%;
  min-height: 72px;
  padding: 20px 56px 20px 24px;
  background: linear-gradient(
    135deg,
    rgba(15, 15, 20, 0.8) 0%,
    rgba(20, 15, 30, 0.6) 50%,
    rgba(10, 10, 15, 0.8) 100%
  );
  border: none;
  border-radius: 16px;
  cursor: pointer;
  overflow: hidden;
  text-align: left;
  animation: choice-appear 0.6s ease-out backwards;
  animation-delay: var(--delay, 0s);
  transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
}

@keyframes choice-appear {
  from {
    opacity: 0;
    transform: translateY(20px) scale(0.95);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

.cinematic-choice:hover {
  transform: translateY(-4px) scale(1.02);
  box-shadow:
    0 20px 40px rgba(0, 0, 0, 0.4),
    0 0 60px rgba(168, 85, 247, 0.15),
    inset 0 1px 0 rgba(255, 255, 255, 0.1);
}

.cinematic-choice:active {
  transform: translateY(-2px) scale(1.01);
  transition-duration: 0.1s;
}

/* 选项背景 */
.choice-bg {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    135deg,
    rgba(168, 85, 247, 0.1) 0%,
    transparent 50%,
    rgba(236, 72, 153, 0.05) 100%
  );
  opacity: 0;
  transition: opacity 0.4s ease;
}

.cinematic-choice:hover .choice-bg {
  opacity: 1;
}

/* 闪光效果 */
.choice-shine {
  position: absolute;
  top: 0;
  left: -100%;
  width: 100%;
  height: 100%;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(255, 255, 255, 0.1) 50%,
    transparent 100%
  );
  transition: left 0.6s ease;
}

.cinematic-choice:hover .choice-shine {
  left: 100%;
}

/* 胶片孔装饰 */
.film-perforation-left,
.film-perforation-right {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  display: flex;
  flex-direction: column;
  gap: 6px;
  pointer-events: none;
}

.film-perforation-left {
  left: 8px;
}

.film-perforation-right {
  right: 8px;
}

.film-hole {
  width: 6px;
  height: 4px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 2px;
  transition: background 0.3s ease;
}

.cinematic-choice:hover .film-hole {
  background: rgba(168, 85, 247, 0.4);
}

/* 内容区域 */
.choice-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  position: relative;
  z-index: 1;
}

.choice-text {
  flex: 1;
}

.text-gradient {
  font-size: 17px;
  font-weight: 300;
  letter-spacing: 0.3px;
  line-height: 1.5;
  color: rgba(255, 255, 255, 0.85);
  transition: all 0.3s ease;
  display: block;
}

.cinematic-choice:hover .text-gradient {
  color: rgba(255, 255, 255, 1);
  text-shadow: 0 0 30px rgba(168, 85, 247, 0.3);
}

/* 交互图标 */
.choice-icon {
  position: relative;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.icon-ring {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  border: 1px solid rgba(168, 85, 247, 0.3);
  opacity: 0;
  transform: scale(0.8);
  transition: all 0.4s ease;
}

.cinematic-choice:hover .icon-ring {
  opacity: 1;
  transform: scale(1);
}

.chevron {
  color: rgba(255, 255, 255, 0.3);
  transition: all 0.3s ease;
  position: relative;
  z-index: 1;
}

.cinematic-choice:hover .chevron {
  color: rgba(168, 85, 247, 1);
  transform: translateX(4px);
}

/* 悬停光效 */
.choice-glow {
  position: absolute;
  inset: -2px;
  border-radius: 18px;
  background: linear-gradient(
    135deg,
    rgba(168, 85, 247, 0.5),
    rgba(236, 72, 153, 0.3),
    rgba(59, 130, 246, 0.5)
  );
  opacity: 0;
  z-index: -1;
  filter: blur(12px);
  transition: opacity 0.4s ease;
}

.cinematic-choice:hover .choice-glow {
  opacity: 0.6;
}

/* 顶部边框装饰 */
.cinematic-choice::before {
  content: '';
  position: absolute;
  top: 0;
  left: 20%;
  width: 60%;
  height: 1px;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(168, 85, 247, 0.3) 50%,
    transparent 100%
  );
  opacity: 0;
  transition: opacity 0.4s ease;
}

.cinematic-choice:hover::before {
  opacity: 1;
}

/* 底部边框装饰 */
.cinematic-choice::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 10%;
  width: 80%;
  height: 1px;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(236, 72, 153, 0.2) 50%,
    transparent 100%
  );
  opacity: 0;
  transition: opacity 0.4s ease 0.1s;
}

.cinematic-choice:hover::after {
  opacity: 1;
}

/* 响应式调整 */
@media (max-width: 768px) {
  .cinematic-choice {
    min-height: 64px;
    padding: 16px 48px 16px 20px;
  }

  .text-gradient {
    font-size: 15px;
  }
}

/* TransitionGroup 动画 - 确保旧选项先消失再显示新选项 */
.choice-enter-active {
  transition: all 0.4s ease-out;
  transition-delay: 0.3s;
}

.choice-leave-active {
  transition: all 0.25s ease-in;
  position: absolute;
  width: 100%;
}

.choice-enter-from {
  opacity: 0;
  transform: translateY(16px) scale(0.95);
}

.choice-leave-to {
  opacity: 0;
  transform: translateY(-8px) scale(0.98);
}
</style>
