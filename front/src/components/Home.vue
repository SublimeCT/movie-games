<script setup lang="ts">
import { useStorage } from '@vueuse/core';
import {
  AlertCircle,
  ClipboardCopy,
  HelpCircle,
  Import as ImportIcon,
  KeyRound,
  Link2,
  Settings as SettingsIcon,
  Sparkles,
  Wand2,
  X,
} from 'lucide-vue-next';
import { ref, watch } from 'vue';
import {
  ApiError,
  type CharacterInput,
  expandCharacter,
  expandSynopsis,
  generateGame,
  generatePrompt,
} from '../api';
import type { MovieTemplate } from '../types/movie';
import CinematicLoader from './ui/CinematicLoader.vue';
import { FluidCursor } from './ui/fluid-cursor';
import { WavyBackground } from './ui/wavy-background';

const emit = defineEmits<(e: 'start', data: MovieTemplate) => void>();

// Persisted State using useStorage
/** Game mode selection: 'wizard' for guided creation, 'free' for free text input */
const mode = useStorage<'wizard' | 'free'>('mg_mode', 'wizard');
/** The main theme or topic of the movie game */
const theme = useStorage('mg_theme', '');
/** The detailed synopsis or storyline */
const synopsis = useStorage('mg_synopsis', ''); // Renamed from worldview
/** Selected genres for the movie */
const selectedGenres = useStorage<string[]>('mg_genres', []); // Added genres
/** List of characters involved in the story */
const characters = useStorage<CharacterInput[]>('mg_characters', [
  { name: '主角', description: '故事的核心人物', gender: '男', isMain: true },
]);
/** Free input text for 'free' mode */
const freeInput = useStorage('mg_free_input', '');
/** API key for GLM service */
const glmApiKey = useStorage('mg_glm_api_key', '');
/** Base URL for GLM service */
const glmBaseUrl = useStorage(
  'mg_glm_base_url',
  'https://open.bigmodel.cn/api/paas/v4/chat/completions',
);
/** Selected GLM model */
const glmModel = useStorage('mg_glm_model', 'glm-4.6v-flash');

// Patch legacy data missing gender
characters.value.forEach((c) => {
  if (!c.gender) c.gender = '其他';
});

// UI State (not persisted)
const isLoading = ref(false);
const isExpandingSyn = ref(false); // Renamed
const isExpandingChar = ref(false);
const error = ref('');
const apiKeyRequired = ref(false);

const isPromptOpen = ref(false);
const isPromptLoading = ref(false);
const promptText = ref('');

const isImportOpen = ref(false);
const importTab = ref<'paste' | 'file'>('paste');
const importText = ref('');
const importError = ref('');
const isHelpOpen = ref(false);

const isSettingsOpen = ref(false);
const baseUrlError = ref('');

/**
 * Validates the custom Base URL.
 * Checks if the URL is well-formed.
 * @returns {boolean} True if valid or empty, false otherwise.
 */
const validateBaseUrl = () => {
  const url = glmBaseUrl.value.trim();
  if (!url) {
    baseUrlError.value = '';
    return true;
  }
  try {
    new URL(url);
    baseUrlError.value = '';
    return true;
  } catch {
    baseUrlError.value = '请输入有效的 URL (例如 https://api.example.com)';
    return false;
  }
};

watch(glmBaseUrl, () => {
  if (baseUrlError.value) validateBaseUrl();
});

const availableGenres = [
  '科幻',
  '剧情',
  '爱情',
  '悬疑',
  '喜剧',
  '青春',
  '历史',
  '冒险',
  '武侠',
  '伦理',
  '悲剧',
  '职场',
];
const customGenre = ref('');

const toggleGenre = (g: string) => {
  if (selectedGenres.value.includes(g)) {
    selectedGenres.value = selectedGenres.value.filter((item) => item !== g);
  } else {
    selectedGenres.value.push(g);
  }
};

const addCustomGenre = () => {
  if (customGenre.value && !selectedGenres.value.includes(customGenre.value)) {
    selectedGenres.value.push(customGenre.value);
    customGenre.value = '';
  }
};

const addCharacter = () => {
  characters.value.push({
    name: '',
    description: '',
    gender: '其他',
    isMain: false,
  });
};

const removeCharacter = (index: number) => {
  characters.value.splice(index, 1);
};

/**
 * Expands the synopsis based on the theme using AI.
 * Requires API key.
 */
const handleExpandSynopsis = async () => {
  const apiKey = glmApiKey.value.trim();
  const baseUrl = glmBaseUrl.value.trim();
  const model = glmModel.value.trim();
  if (!theme.value) {
    error.value = '请先填写主题';
    return;
  }
  isExpandingSyn.value = true;
  try {
    const text = await expandSynopsis(
      theme.value,
      synopsis.value,
      navigator.language,
      apiKey || undefined,
      baseUrl || undefined,
      model || undefined,
    );
    synopsis.value = text;
    // biome-ignore lint/suspicious/noExplicitAny: Error handling
  } catch (e: any) {
    if (e instanceof ApiError && e.code === 'API_KEY_REQUIRED') {
      isSettingsOpen.value = true;
      apiKeyRequired.value = true;
      error.value = e.message;
    } else {
      error.value = '扩写失败，请重试';
    }
  } finally {
    isExpandingSyn.value = false;
  }
};

/**
 * Generates characters based on the theme and synopsis using AI.
 * Requires API key and non-empty synopsis.
 */
const handleExpandCharacter = async () => {
  const apiKey = glmApiKey.value.trim();
  const baseUrl = glmBaseUrl.value.trim();
  const model = glmModel.value.trim();
  if (!theme.value || !synopsis.value) {
    error.value = '请先填写主题和剧情简介';
    return;
  }
  isExpandingChar.value = true;
  try {
    const newChars = await expandCharacter(
      theme.value,
      synopsis.value,
      characters.value,
      navigator.language,
      apiKey || undefined,
      baseUrl || undefined,
      model || undefined,
    );
    characters.value = newChars;
    // biome-ignore lint/suspicious/noExplicitAny: Error handling
  } catch (e: any) {
    if (e instanceof ApiError && e.code === 'API_KEY_REQUIRED') {
      isSettingsOpen.value = true;
      apiKeyRequired.value = true;
      error.value = e.message;
    } else {
      error.value = '角色生成失败';
    }
  } finally {
    isExpandingChar.value = false;
  }
};

/**
 * Selects the optimal image size for CogView based on the current viewport aspect ratio.
 * @returns {'1024x1024' | '864x1152' | '1152x864'} The best matching size string.
 */
const selectCogViewSize = (): '1024x1024' | '864x1152' | '1152x864' => {
  const vw = window.visualViewport?.width ?? window.innerWidth;
  const vh = window.visualViewport?.height ?? window.innerHeight;
  const viewportRatio = vh > 0 ? vw / vh : 1;

  const candidates = [
    { size: '1024x1024' as const, w: 1024, h: 1024 },
    { size: '1152x864' as const, w: 1152, h: 864 },
    { size: '864x1152' as const, w: 864, h: 1152 },
  ];

  const [first, ...rest] = candidates;
  if (!first) return '1024x1024';

  let best = first;
  let bestDiff = Math.abs(viewportRatio - best.w / best.h);
  let bestPixels = best.w * best.h;

  for (const c of rest) {
    const diff = Math.abs(viewportRatio - c.w / c.h);
    const pixels = c.w * c.h;
    if (diff < bestDiff || (diff === bestDiff && pixels > bestPixels)) {
      best = c;
      bestDiff = diff;
      bestPixels = pixels;
    }
  }

  return best.size;
};

/**
 * Main function to generate the game.
 * Calls the API to generate the full game data structure.
 * Handles loading states and errors.
 */
const handleGenerate = async () => {
  const apiKey = glmApiKey.value.trim();
  const baseUrl = glmBaseUrl.value.trim();
  const model = glmModel.value.trim();
  isLoading.value = true;
  error.value = '';
  apiKeyRequired.value = false;
  try {
    const size = selectCogViewSize();
    const data = await generateGame({
      mode: mode.value,
      theme: theme.value,
      synopsis: synopsis.value,
      genre: selectedGenres.value, // Added genres
      characters: characters.value,
      freeInput: freeInput.value,
      language: navigator.language,
      size,
      apiKey: apiKey || undefined,
      baseUrl: baseUrl || undefined,
      model: model || undefined,
    });
    emit('start', data);
    // biome-ignore lint/suspicious/noExplicitAny: Error handling
  } catch (e: any) {
    if (e instanceof ApiError && e.code === 'API_KEY_REQUIRED') {
      isSettingsOpen.value = true;
      apiKeyRequired.value = true;
    }
    error.value = e.message || '生成失败，请检查网络或重试';
  } finally {
    isLoading.value = false;
  }
};

const handleGeneratePrompt = async () => {
  const apiKey = glmApiKey.value.trim();
  const baseUrl = glmBaseUrl.value.trim();
  const model = glmModel.value.trim();
  isPromptLoading.value = true;
  error.value = '';
  try {
    const size = selectCogViewSize();
    const text = await generatePrompt({
      mode: mode.value,
      theme: theme.value,
      synopsis: synopsis.value,
      genre: selectedGenres.value,
      characters: characters.value,
      freeInput: freeInput.value,
      language: navigator.language,
      size,
      apiKey: apiKey || undefined,
      baseUrl: baseUrl || undefined,
      model: model || undefined,
    });
    promptText.value = text;
    isPromptOpen.value = true;
    // biome-ignore lint/suspicious/noExplicitAny: Error handling
  } catch (e: any) {
    if (e instanceof ApiError && e.code === 'API_KEY_REQUIRED') {
      isSettingsOpen.value = true;
      apiKeyRequired.value = true;
    }
    error.value = e.message || '获取提示词失败';
  } finally {
    isPromptLoading.value = false;
  }
};

const openImport = () => {
  importError.value = '';
  importText.value = '';
  importTab.value = 'paste';
  isImportOpen.value = true;
};

const onImportFile = (e: Event) => {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  importError.value = '';
  const reader = new FileReader();
  reader.onload = () => {
    importText.value = String(reader.result || '');
  };
  reader.onerror = () => {
    importError.value = '读取文件失败';
  };
  reader.readAsText(file);
};

const confirmImport = () => {
  importError.value = '';
  try {
    const raw = importText.value.trim();
    if (!raw) {
      importError.value = '请粘贴或上传 JSON';
      return;
    }
    const data = JSON.parse(raw) as MovieTemplate;
    // biome-ignore lint/suspicious/noExplicitAny: Dynamic data
    const nodes = (data as any)?.nodes;
    // biome-ignore lint/suspicious/noExplicitAny: Dynamic data
    const endings = (data as any)?.endings;
    if (!nodes || typeof nodes !== 'object') {
      importError.value = 'JSON 缺少 nodes';
      return;
    }
    if (!endings || typeof endings !== 'object') {
      importError.value = 'JSON 缺少 endings';
      return;
    }
    isImportOpen.value = false;
    emit('start', data);
  } catch {
    importError.value = 'JSON 解析失败，请检查格式';
  }
};

const copyPrompt = async () => {
  try {
    await navigator.clipboard.writeText(promptText.value || '');
  } catch {
    error.value = '复制失败';
  }
};

const btnRef = ref<HTMLButtonElement | null>(null);
const handleBtnMouseMove = (e: MouseEvent) => {
  if (!btnRef.value) return;
  const rect = btnRef.value.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;
  btnRef.value.style.setProperty('--x', `${x}px`);
  btnRef.value.style.setProperty('--y', `${y}px`);
};
</script>

<template>
  <div class="relative h-screen overflow-hidden w-full cursor-auto">
    <FluidCursor />
    <CinematicLoader v-if="isLoading" />

    <!-- Prompt Modal -->
    <Transition enter-active-class="animate-in fade-in duration-200" leave-active-class="animate-out fade-out duration-150">
      <div v-if="isPromptOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4">
        <div class="absolute inset-0 bg-black/80 backdrop-blur-md" @click="isPromptOpen = false"></div>
        <div class="w-full max-w-4xl bg-neutral-900/90 border border-white/10 rounded-2xl overflow-hidden shadow-2xl relative z-10">
          <div class="px-5 py-4 flex items-center justify-between border-b border-white/10">
            <div class="text-sm tracking-[0.22em] uppercase text-white/70 font-semibold flex items-center gap-2">
              <Sparkles class="w-4 h-4 text-purple-400" />
              生成提示词
            </div>
            <button @click="isPromptOpen = false" class="p-2 rounded-lg hover:bg-white/5 transition-colors">
              <X class="w-5 h-5 text-white/70" />
            </button>
          </div>
          <div class="p-5">
            <textarea
              :value="promptText"
              readonly
              rows="14"
              class="w-full bg-black/50 border border-neutral-800 rounded-xl px-4 py-3 text-sm text-white/90 focus:ring-2 focus:ring-purple-500 focus:border-transparent outline-none placeholder-neutral-600 transition-all resize-none font-mono leading-relaxed"
              placeholder="(空)"
            />
            <div class="mt-4 flex items-center justify-end gap-3">
              <button @click="copyPrompt" class="relative inline-flex items-center gap-2 px-4 py-2 rounded-full border border-white/10 bg-black/30 backdrop-blur-md text-sm font-semibold text-white/80 hover:text-white hover:border-purple-500/50 transition-all overflow-hidden group">
                <span class="absolute inset-0 bg-gradient-to-r from-purple-500/10 via-pink-500/10 to-cyan-400/10 opacity-60 group-hover:opacity-100 transition-opacity"></span>
                <span class="relative inline-flex items-center gap-2">
                  <ClipboardCopy class="w-4 h-4" />
                  <span class="tracking-wide">复制</span>
                </span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </Transition>

    <!-- Help Modal -->
    <Transition enter-active-class="animate-in fade-in duration-200" leave-active-class="animate-out fade-out duration-150">
      <div v-if="isHelpOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4">
        <div class="absolute inset-0 bg-black/80 backdrop-blur-md" @click="isHelpOpen = false"></div>
        <div class="w-full max-w-2xl bg-neutral-900/90 border border-white/10 rounded-2xl overflow-hidden shadow-2xl relative z-10">
          <div class="px-6 py-5 flex items-center justify-between border-b border-white/10 bg-gradient-to-r from-purple-900/20 to-transparent">
            <div class="flex items-center gap-3">
              <div class="p-2 rounded-lg bg-purple-500/20">
                <Sparkles class="w-5 h-5 text-purple-300" />
              </div>
              <div>
                <h3 class="text-lg font-bold text-white tracking-tight">设计理念</h3>
                <p class="text-xs text-white/50 uppercase tracking-widest font-semibold">打造沉浸式电影体验</p>
              </div>
            </div>
            <button @click="isHelpOpen = false" class="p-2 rounded-full hover:bg-white/10 transition-colors">
              <X class="w-5 h-5 text-white/60" />
            </button>
          </div>
          
          <div class="p-8 max-h-[70vh] overflow-y-auto space-y-8 custom-scrollbar">
            <div class="space-y-4">
              <h4 class="text-sm font-bold text-purple-400 uppercase tracking-widest border-b border-purple-500/20 pb-2 mb-4">核心机制</h4>
              <p class="text-neutral-300 leading-relaxed">
                <strong class="text-white">Movie Games</strong> 将您的创意转化为互动电影。通过定义主题、角色和关键情节，我们的 AI 引擎将构建一个包含分支剧情和多重结局的复杂叙事树。
              </p>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
              <div class="space-y-3">
                <div class="w-10 h-10 rounded-full bg-blue-500/10 flex items-center justify-center border border-blue-500/20">
                  <Wand2 class="w-5 h-5 text-blue-400" />
                </div>
                <h5 class="text-white font-bold">AI 智能扩写</h5>
                <p class="text-sm text-neutral-400 leading-relaxed">
                  使用 <span class="text-blue-300">AI 智能扩写</span> 按钮，根据您的主题自动丰富故事大纲，或生成具有深度和矛盾冲突的角色。
                </p>
              </div>

              <div class="space-y-3">
                <div class="w-10 h-10 rounded-full bg-pink-500/10 flex items-center justify-center border border-pink-500/20">
                  <ImportIcon class="w-5 h-5 text-pink-400" />
                </div>
                <h5 class="text-white font-bold">JSON 导入</h5>
                <p class="text-sm text-neutral-400 leading-relaxed">
                  已有剧本？导入现有的 <code class="bg-neutral-800 px-1 py-0.5 rounded text-xs text-pink-300">MovieTemplate</code> JSON 文件，即可立即可视化或重制您的故事。
                </p>
              </div>
            </div>

            <div class="space-y-4">
               <h4 class="text-sm font-bold text-purple-400 uppercase tracking-widest border-b border-purple-500/20 pb-2 mb-4">最佳效果技巧</h4>
               <ul class="space-y-3 text-neutral-300 text-sm">
                 <li class="flex gap-3">
                   <span class="w-1.5 h-1.5 rounded-full bg-purple-500 mt-2 flex-shrink-0"></span>
                   <span>提供详细的<strong>故事大纲</strong>。您提供的上下文越多，生成的剧情就越连贯。</span>
                 </li>
                 <li class="flex gap-3">
                   <span class="w-1.5 h-1.5 rounded-full bg-purple-500 mt-2 flex-shrink-0"></span>
                   <span>创建至少 <strong>3 个角色</strong>以获得丰富的人物互动。在描述中定义他们的“深层需求”。</span>
                 </li>
                 <li class="flex gap-3">
                   <span class="w-1.5 h-1.5 rounded-full bg-purple-500 mt-2 flex-shrink-0"></span>
                   <span>如果您想在发送给 AI 之前检查或手动调整提示词，请使用<strong>仅生成提示词</strong>功能。</span>
                 </li>
               </ul>
            </div>
          </div>

          <div class="px-6 py-4 bg-black/20 border-t border-white/5 flex justify-end">
            <button @click="isHelpOpen = false" class="px-6 py-2 rounded-xl bg-white/10 hover:bg-white/20 text-white font-medium transition-colors">
              知道了
            </button>
          </div>
        </div>
      </div>
    </Transition>

    <!-- Import Modal -->
    <Transition enter-active-class="animate-in fade-in duration-200" leave-active-class="animate-out fade-out duration-150">
      <div v-if="isImportOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4">
        <div class="absolute inset-0 bg-black/80 backdrop-blur-md" @click="isImportOpen = false"></div>
        <div class="w-full max-w-3xl bg-neutral-900/90 border border-white/10 rounded-2xl overflow-hidden shadow-2xl relative z-10">
          <div class="px-5 py-4 flex items-center justify-between border-b border-white/10">
            <div class="text-sm tracking-[0.22em] uppercase text-white/70 font-semibold flex items-center gap-2">
              <ImportIcon class="w-4 h-4 text-cyan-400" />
              导入游戏数据
            </div>
            <button @click="isImportOpen = false" class="p-2 rounded-lg hover:bg-white/5 transition-colors">
              <X class="w-5 h-5 text-white/70" />
            </button>
          </div>
          <div class="p-5">
            <div class="flex items-center gap-2 mb-4">
              <button
                @click="importTab = 'paste'"
                :class="['px-4 py-2 rounded-full text-sm font-semibold border transition-all', importTab === 'paste' ? 'bg-purple-600/30 border-purple-500/40 text-white' : 'bg-black/30 border-white/10 text-white/60 hover:text-white hover:border-purple-500/30']"
              >
                手动输入
              </button>
              <button
                @click="importTab = 'file'"
                :class="['px-4 py-2 rounded-full text-sm font-semibold border transition-all', importTab === 'file' ? 'bg-purple-600/30 border-purple-500/40 text-white' : 'bg-black/30 border-white/10 text-white/60 hover:text-white hover:border-purple-500/30']"
              >
                上传文件
              </button>
            </div>

            <div v-if="importTab === 'paste'" class="space-y-3">
              <textarea
                v-model="importText"
                rows="12"
                class="w-full bg-black/50 border border-neutral-800 rounded-xl px-4 py-3 text-sm text-white/90 focus:ring-2 focus:ring-purple-500 focus:border-transparent outline-none placeholder-neutral-600 transition-all resize-none font-mono leading-relaxed"
                placeholder="粘贴之前导出的 MovieTemplate JSON"
              />
            </div>

            <div v-else class="space-y-3">
              <input
                type="file"
                accept="application/json,.json"
                @change="onImportFile"
                class="block w-full text-sm text-white/70 file:mr-4 file:py-2 file:px-4 file:rounded-full file:border-0 file:text-sm file:font-semibold file:bg-purple-600/30 file:text-white hover:file:bg-purple-600/40"
              />
              <textarea
                v-model="importText"
                rows="10"
                class="w-full bg-black/50 border border-neutral-800 rounded-xl px-4 py-3 text-sm text-white/90 focus:ring-2 focus:ring-purple-500 focus:border-transparent outline-none placeholder-neutral-600 transition-all resize-none font-mono leading-relaxed"
                placeholder="文件内容会显示在这里"
              />
            </div>

            <div v-if="importError" class="mt-4 bg-red-500/10 border border-red-500/20 text-red-500 p-3 rounded-xl text-sm text-center">{{ importError }}</div>

            <div class="mt-5 flex items-center justify-end gap-3">
              <button @click="confirmImport" class="relative inline-flex items-center gap-2 px-5 py-2.5 rounded-full bg-gradient-to-r from-purple-600 to-pink-600 text-white font-bold hover:shadow-[0_0_30px_rgba(168,85,247,0.35)] hover:scale-[1.01] active:scale-[0.99] transition-all">
                <ImportIcon class="w-4 h-4" />
                导入并开始
              </button>
            </div>
          </div>
        </div>
      </div>
    </Transition>

    <!-- Settings Modal -->
    <Transition enter-active-class="animate-in fade-in duration-200" leave-active-class="animate-out fade-out duration-150">
      <div v-if="isSettingsOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4">
        <div class="absolute inset-0 bg-black/80 backdrop-blur-md" @click="isSettingsOpen = false"></div>
        <div class="w-full max-w-2xl bg-neutral-900/90 border border-white/10 rounded-2xl overflow-hidden shadow-2xl relative z-10">
          <div class="px-5 py-4 flex items-center justify-between border-b border-white/10">
            <div class="text-sm tracking-[0.22em] uppercase text-white/70 font-semibold flex items-center gap-2">
              <SettingsIcon class="w-4 h-4 text-white/70" />
              connection settings
            </div>
            <button @click="isSettingsOpen = false" class="p-2 rounded-lg hover:bg-white/5 transition-colors">
              <X class="w-5 h-5 text-white/70" />
            </button>
          </div>
          <div class="p-8 space-y-6">
            <div class="space-y-3">
                <div class="flex items-center justify-between">
                <label class="text-sm font-bold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                    <KeyRound class="w-4 h-4 text-purple-400" />
                    API Key
                </label>
                <div v-if="apiKeyRequired" class="text-xs text-pink-500 font-bold bg-pink-500/10 px-2 py-0.5 rounded animate-pulse">需要填写 API Key 才能继续</div>
                </div>
                <input
                v-model="glmApiKey"
                type="password"
                autocomplete="off"
                spellcheck="false"
                class="w-full bg-black/50 border border-neutral-700 rounded-xl px-4 py-3 text-white focus:ring-2 focus:ring-purple-500 focus:border-transparent outline-none placeholder-neutral-600 transition-all font-mono"
                placeholder="不填则使用服务端默认 Key"
                />
                <p class="text-xs text-neutral-500">
                    默认情况下无需填写。当每日访问量超过限制或并发较高时，系统会提示您填写。
                </p>
            </div>

            <div class="space-y-3">
                <label class="text-sm font-bold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                    <Link2 class="w-4 h-4 text-cyan-400" />
                    Base URL
                </label>
                <input
                v-model="glmBaseUrl"
                @blur="validateBaseUrl"
                type="text"
                autocomplete="off"
                spellcheck="false"
                :class="['w-full bg-black/50 border rounded-xl px-4 py-3 text-white focus:ring-2 focus:ring-purple-500 focus:border-transparent outline-none placeholder-neutral-600 transition-all font-mono', baseUrlError ? 'border-red-500/50 focus:ring-red-500' : 'border-neutral-700']"
                placeholder="可选：自定义 GLM 接口 Base URL"
                />
                <p v-if="baseUrlError" class="text-xs text-red-400 font-bold">{{ baseUrlError }}</p>
                <p v-else class="text-xs text-neutral-500">
                    如果您使用中转服务或自定义代理，请在此填写完整的 Base URL。
                </p>
            </div>

            <div class="space-y-3">
                <label class="text-sm font-bold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                    <Wand2 class="w-4 h-4 text-pink-400" />
                    Model
                </label>
                <input
                v-model="glmModel"
                type="text"
                autocomplete="off"
                spellcheck="false"
                class="w-full bg-black/50 border border-neutral-700 rounded-xl px-4 py-3 text-white focus:ring-2 focus:ring-purple-500 focus:border-transparent outline-none placeholder-neutral-600 transition-all font-mono"
                placeholder="glm-4.6v-flash"
                />
                <p class="text-xs text-neutral-500">
                    指定使用的模型名称（默认为 glm-4.6v-flash）。如果不填写，将使用默认值。
                </p>
            </div>

            <div class="pt-4 flex justify-end">
                <button @click="isSettingsOpen = false" class="px-6 py-2 rounded-full bg-white text-black font-bold hover:bg-neutral-200 transition-colors">
                    完成
                </button>
            </div>
          </div>
        </div>
      </div>
    </Transition>
    
    <!-- Inspira-style Background -->
    <WavyBackground 
      container-class="fixed inset-0 z-0 pointer-events-none"
      :colors="['#38bdf8', '#818cf8', '#c084fc', '#e879f9', '#22d3ee']"
      :waveWidth="100"
      :blur="20"
      speed="fast"
    />

    <div class="container mx-auto h-full max-w-4xl animate-fade-in text-neutral-100 relative z-10 flex flex-col px-4 py-8">
        <!-- Header -->
        <header class="mb-6 md:mb-12 relative flex-shrink-0">
            <div class="absolute top-0 right-0 flex items-center gap-3">
                <button
                    @click="isHelpOpen = true"
                    class="p-2 rounded-full bg-black/30 backdrop-blur-md border border-white/10 hover:bg-white/10 hover:border-purple-500/50 transition-all group"
                    title="帮助"
                >
                    <HelpCircle class="w-5 h-5 text-white/70 group-hover:text-white transition-colors" />
                </button>
                <button
                    @click="openImport"
                    class="p-2 rounded-full bg-black/30 backdrop-blur-md border border-white/10 hover:bg-white/10 hover:border-purple-500/50 transition-all group"
                    title="导入存档"
                >
                    <ImportIcon class="w-5 h-5 text-white/70 group-hover:text-white transition-colors" />
                </button>
                <button
                    @click="isSettingsOpen = true"
                    class="p-2 rounded-full bg-black/30 backdrop-blur-md border border-white/10 hover:bg-white/10 hover:border-purple-500/50 transition-all group"
                    title="连接设置"
                >
                    <SettingsIcon class="w-5 h-5 text-white/70 group-hover:text-white transition-colors" />
                </button>
            </div>

            <div class="text-center pt-8">
                <h1 class="text-6xl font-bold bg-gradient-to-r from-purple-400 via-pink-500 to-cyan-400 bg-clip-text text-transparent mb-4 tracking-tight animate-pulse-slow drop-shadow-[0_0_15px_rgba(168,85,247,0.5)]">
                    无限互动电影
                </h1>
                <p class="text-neutral-400 text-lg tracking-[0.2em] uppercase font-light">AI 驱动的互动小说游戏生成器</p>
            </div>
        </header>

        <!-- Main Card with Glow Effect -->
        <div class="relative group flex-shrink min-h-0 flex flex-col justify-center max-h-full">
            <!-- Glow Border -->
            <div class="absolute -inset-0.5 bg-gradient-to-r from-purple-600 via-pink-600 to-cyan-600 rounded-2xl blur opacity-30 group-hover:opacity-70 transition duration-1000 group-hover:duration-200 animate-tilt"></div>
            
            <div class="relative bg-black/80 backdrop-blur-xl border border-neutral-800 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-full md:h-auto md:max-h-[80vh]">
                <!-- Inner Decoration -->
                <div class="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-transparent via-white/10 to-transparent z-10"></div>

            <!-- Wizard Mode Content -->
            <div class="p-8 md:p-12 overflow-y-auto flex-1 no-scrollbar space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
                <!-- Theme -->
                <div class="space-y-3">
                <label class="text-lg font-bold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                    <span class="w-1.5 h-1.5 rounded-full bg-purple-500"></span>
                    游戏主题
                </label>
                <input v-model="theme" type="text" class="w-full bg-neutral-900/50 border border-neutral-700 rounded-xl px-5 py-4 text-lg focus:ring-2 focus:ring-purple-500 focus:border-transparent outline-none text-white placeholder-neutral-600 transition-all" placeholder="例如：赛博朋克背景下的硬汉侦探故事...">
                </div>

                <!-- Genre Selection -->
                <div class="space-y-4">
                    <label class="text-lg font-bold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                        <span class="w-1.5 h-1.5 rounded-full bg-pink-500"></span>
                        剧情类型 (多选)
                    </label>
                    <div class="flex flex-wrap gap-3">
                        <button 
                            v-for="g in availableGenres" 
                            :key="g"
                            @click="toggleGenre(g)"
                            :class="['px-4 py-2 rounded-lg text-base transition-all border', selectedGenres.includes(g) ? 'bg-purple-600 border-purple-500 text-white shadow-lg shadow-purple-900/50' : 'bg-neutral-900 border-neutral-700 text-neutral-400 hover:border-purple-500/50']"
                        >
                            {{ g }}
                        </button>
                        <div class="flex items-center gap-2">
                            <input 
                                v-model="customGenre" 
                                @keydown.enter="addCustomGenre"
                                placeholder="添加..." 
                                class="px-4 py-2 rounded-lg text-base bg-neutral-900 border border-neutral-700 text-white focus:border-purple-500 outline-none w-24 focus:w-32 transition-all"
                            />
                            <button @click="addCustomGenre" class="text-purple-400 hover:text-white text-xl px-2">+</button>
                        </div>
                    </div>
                </div>

                <!-- Synopsis (Renamed from Worldview) -->
                <div class="space-y-3">
                <div class="flex justify-between items-center">
                    <label class="text-lg font-bold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                        <span class="w-1.5 h-1.5 rounded-full bg-cyan-500"></span>
                        剧情简介
                    </label>
                    <button @click="handleExpandSynopsis" :disabled="isExpandingSyn" class="relative inline-flex items-center gap-2 px-6 py-2.5 rounded-full border border-purple-500/50 bg-purple-500/10 backdrop-blur-md text-sm font-bold text-purple-200 hover:text-white hover:bg-purple-500/20 hover:border-purple-400 transition-all disabled:opacity-50 overflow-hidden group shadow-[0_0_15px_rgba(168,85,247,0.15)] hover:shadow-[0_0_25px_rgba(168,85,247,0.3)]">
                        <span class="absolute inset-0 bg-gradient-to-r from-transparent via-white/10 to-transparent -translate-x-full group-hover:animate-shimmer"></span>
                        <span class="relative inline-flex items-center gap-2">
                          <span v-if="isExpandingSyn" class="inline-flex items-center gap-1">
                            <span class="w-1.5 h-1.5 rounded-full bg-purple-300 animate-pulse"></span>
                            <span class="w-1.5 h-1.5 rounded-full bg-pink-300 animate-pulse" :style="{ animationDelay: '120ms' }"></span>
                            <span class="w-1.5 h-1.5 rounded-full bg-cyan-200 animate-pulse" :style="{ animationDelay: '240ms' }"></span>
                          </span>
                          <Wand2 v-else class="w-4 h-4 text-purple-400 group-hover:text-white transition-colors" />
                          <span class="tracking-wide uppercase">AI 智能扩写</span>
                        </span>
                    </button>
                </div>
                <div class="relative">
                    <textarea 
                        v-model="synopsis" 
                        :disabled="isExpandingSyn"
                        rows="6" 
                        :class="['w-full bg-neutral-900/50 border rounded-xl px-5 py-4 text-lg outline-none text-white placeholder-neutral-600 transition-all resize-none leading-relaxed', isExpandingSyn ? 'border-purple-500/50 animate-pulse bg-purple-900/10' : 'border-neutral-700 focus:ring-2 focus:ring-purple-500 focus:border-transparent']"
                        placeholder="描述故事的核心冲突、世界背景和开场氛围..."
                    ></textarea>
                    <div v-if="isExpandingSyn" class="absolute inset-0 flex items-center justify-center pointer-events-none">
                        <span class="bg-black/50 backdrop-blur px-4 py-2 rounded-lg text-purple-300 text-sm font-bold animate-pulse">AI 正在扩写中...</span>
                    </div>
                </div>
                </div>

                <!-- Characters -->
                <div class="space-y-5">
                <div class="flex justify-between items-center">
                    <label class="text-lg font-bold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                        <span class="w-1.5 h-1.5 rounded-full bg-yellow-500"></span>
                        角色阵容
                    </label>
                    <div class="flex gap-3">
                        <button @click="handleExpandCharacter" :disabled="isExpandingChar" class="relative inline-flex items-center gap-2 px-6 py-2.5 rounded-full border border-purple-500/30 bg-purple-900/20 backdrop-blur-md text-sm font-semibold text-purple-200 hover:text-white hover:bg-purple-900/40 hover:border-purple-500/60 transition-all disabled:opacity-50 overflow-hidden group">
                            <span class="absolute inset-0 bg-gradient-to-r from-purple-500/10 via-pink-500/10 to-cyan-400/10 opacity-0 group-hover:opacity-100 transition-opacity"></span>
                            <span class="relative inline-flex items-center gap-2">
                              <span v-if="isExpandingChar" class="inline-flex items-center gap-1">
                                <span class="w-1.5 h-1.5 rounded-full bg-purple-300 animate-pulse"></span>
                                <span class="w-1.5 h-1.5 rounded-full bg-pink-300 animate-pulse" :style="{ animationDelay: '120ms' }"></span>
                                <span class="w-1.5 h-1.5 rounded-full bg-cyan-200 animate-pulse" :style="{ animationDelay: '240ms' }"></span>
                              </span>
                              <Sparkles v-else class="w-4 h-4" />
                              <span class="tracking-wide uppercase">AI 生成角色</span>
                            </span>
                        </button>
                        <button @click="addCharacter" class="relative inline-flex items-center gap-2 px-6 py-2.5 rounded-full border border-white/10 bg-black/30 backdrop-blur-md text-sm font-semibold text-white/70 hover:text-white hover:border-white/30 transition-all overflow-hidden">
                          <span class="tracking-wide">+ 添加角色</span>
                        </button>
                    </div>
                </div>
                <div class="relative">
                    <div :class="['grid gap-4', isExpandingChar ? 'opacity-50 pointer-events-none blur-sm' : '']">
                        <div v-for="(char, idx) in characters" :key="idx" class="bg-neutral-900/50 p-5 rounded-xl border border-neutral-800 flex flex-col gap-4 relative group/item hover:border-purple-500/30 transition-all">
                            <div class="flex flex-wrap md:flex-nowrap gap-4 items-center w-full">
                                <input v-model="char.name" placeholder="名字" class="w-full md:w-1/4 bg-neutral-950 border border-neutral-800 rounded-lg px-4 py-3 text-base text-white focus:border-purple-500 outline-none transition-colors">
                                <input v-model="char.gender" placeholder="性别" class="w-full md:w-28 bg-neutral-950 border border-neutral-800 rounded-lg px-4 py-3 text-base text-white focus:border-purple-500 outline-none transition-colors">
                                <label class="flex-shrink-0 flex items-center justify-center gap-2 text-sm text-neutral-400 cursor-pointer select-none bg-neutral-950 rounded-lg border border-neutral-800 hover:bg-neutral-900 transition-colors px-4 py-3 whitespace-nowrap">
                                    <input type="checkbox" v-model="char.isMain" class="accent-purple-500 w-4 h-4"> 主角
                                </label>
                                <div class="flex-1"></div>
                                <button @click="removeCharacter(idx)" class="text-neutral-600 hover:text-red-500 p-2 transition-colors text-xl flex-shrink-0">×</button>
                            </div>
                            <textarea v-model="char.description" placeholder="身份与性格描述" rows="2" class="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-4 py-3 text-base text-white focus:border-purple-500 outline-none transition-colors resize-none custom-scrollbar"></textarea>
                        </div>
                    </div>
                    <div v-if="isExpandingChar" class="absolute inset-0 flex items-center justify-center pointer-events-none">
                        <span class="bg-black/50 backdrop-blur px-6 py-3 rounded-xl text-purple-300 text-lg font-bold animate-pulse border border-purple-500/30">AI 正在生成角色...</span>
                    </div>
                </div>
                </div>

            </div>

            <!-- Action -->
            <div class="z-20 bg-black/90 backdrop-blur-xl border-t border-white/10 p-6 shadow-[0_-10px_40px_rgba(0,0,0,0.5)] flex-shrink-0 relative">
                <!-- Error Modal (Replaces inline error) -->
                <Transition enter-active-class="animate-in fade-in duration-200" leave-active-class="animate-out fade-out duration-150">
                    <div v-if="error" class="absolute left-0 right-0 bottom-full mb-4 px-6 flex justify-center pointer-events-none">
                        <div class="w-full max-w-md bg-neutral-900 border border-red-500/30 rounded-2xl p-4 shadow-2xl pointer-events-auto flex items-center gap-4">
                                <div class="w-10 h-10 rounded-full bg-red-500/10 flex-shrink-0 flex items-center justify-center">
                                    <AlertCircle class="w-5 h-5 text-red-500" />
                                </div>
                                <div class="flex-1 min-w-0">
                                  <h3 class="text-sm font-bold text-white">出错啦</h3>
                                  <p class="text-xs text-neutral-300 truncate">{{ error }}</p>
                                </div>
                                <button 
                                    @click="error = ''" 
                                    class="px-3 py-1.5 rounded-lg bg-red-500/10 hover:bg-red-500/20 text-red-400 text-xs font-bold transition-colors border border-red-500/20 whitespace-nowrap"
                                >
                                    知道了
                                </button>
                        </div>
                    </div>
                </Transition>
                
                <button 
                    ref="btnRef"
                    @mousemove="handleBtnMouseMove"
                    @click="handleGenerate" 
                    :disabled="isLoading || isExpandingSyn || isExpandingChar"
                    class="w-full py-5 rounded-xl bg-neutral-900 border border-white/10 text-white font-black text-xl hover:shadow-[0_0_30px_rgba(168,85,247,0.4)] hover:scale-[1.01] active:scale-[0.99] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex justify-center items-center gap-3 relative overflow-hidden group"
                >
                    <!-- Spotlight Effect -->
                    <div class="pointer-events-none absolute -inset-px opacity-0 transition duration-300 group-hover:opacity-100" style="background: radial-gradient(600px circle at var(--x) var(--y), rgba(168, 85, 247, 0.4), transparent 40%);"></div>
                    <!-- Background Gradient (Subtle) -->
                     <div class="absolute inset-0 bg-gradient-to-r from-purple-900/50 via-pink-900/50 to-purple-900/50 opacity-50"></div>

                    <svg v-if="isLoading" viewBox="0 0 24 24" fill="none" class="w-6 h-6 text-white/95 animate-spin relative z-10">
                      <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2" class="opacity-20"/>
                      <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
                    </svg>
                    <span class="relative z-10">{{ isLoading ? '正在生成剧本...' : '🚀 开始生成剧情' }}</span>
                </button>

                <div class="mt-4 flex justify-center">
                  <button
                    @click="handleGeneratePrompt"
                    :disabled="isPromptLoading"
                    class="relative inline-flex items-center gap-2 px-6 py-2 rounded-full border border-white/10 bg-white/5 backdrop-blur-md text-xs font-bold text-white/50 hover:text-white hover:border-white/30 hover:bg-white/10 transition-all disabled:opacity-30 group overflow-hidden"
                  >
                    <span class="absolute inset-0 bg-gradient-to-r from-transparent via-white/5 to-transparent -translate-x-full group-hover:animate-shimmer"></span>
                    <span v-if="isPromptLoading" class="inline-flex items-center gap-1">
                        <span class="w-1.5 h-1.5 rounded-full bg-white/50 animate-pulse"></span>
                        <span class="w-1.5 h-1.5 rounded-full bg-white/50 animate-pulse" :style="{ animationDelay: '120ms' }"></span>
                        <span class="w-1.5 h-1.5 rounded-full bg-white/50 animate-pulse" :style="{ animationDelay: '240ms' }"></span>
                    </span>
                    <Sparkles v-else class="w-3 h-3 text-purple-400 group-hover:text-purple-300 transition-colors" />
                    <span class="tracking-widest uppercase">仅生成提示词</span>
                  </button>
                </div>
            </div>
            </div>
        </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes gradient {
  0% {
    background-position: 0% 50%;
  }
  50% {
    background-position: 100% 50%;
  }
  100% {
    background-position: 0% 50%;
  }
}

.animate-gradient {
  animation: gradient 3s ease infinite;
}
</style>
