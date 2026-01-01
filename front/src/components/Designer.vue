<script setup lang="ts">
import { useStorage } from '@vueuse/core';
import {
  ArrowLeft,
  Copy,
  Download,
  FileJson,
  Home as HomeIcon,
  Import as ImportIcon,
  Lock,
  Pencil,
  Plus,
  Save,
  Share2,
  Spline,
  Trash2,
  X,
} from 'lucide-vue-next';
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import type { CharacterInput as ApiCharacterInput } from '../api';
import {
  ApiError,
  getSharedGame,
  getSharedRecordMeta,
  importGameTemplate,
  shareGame,
  updateGameTemplate,
} from '../api';
import { useGameState } from '../hooks/useGameState';
import type { Choice, Ending, MovieTemplate, StoryNode } from '../types/movie';
import CharacterAvatar from './ui/CharacterAvatar.vue';
import CinematicLoader from './ui/CinematicLoader.vue';
import { WavyBackground } from './ui/wavy-background';

type PlayEntry = 'owner' | 'shared' | 'import';

type TreeNodeKind = 'story' | 'ending';

type TreeNodeVM = {
  id: string;
  kind: TreeNodeKind;
  label: string;
  depth: number;
  x: number;
  y: number;
  w: number;
  h: number;
};

type EdgeVM = {
  from: string;
  to: string;
  label?: string;
};

const router = useRouter();
const route = useRoute();

const { gameData } = useGameState();

const theme = useStorage('mg_theme', '');
const synopsis = useStorage('mg_synopsis', '');
const selectedGenres = useStorage<string[]>('mg_genres', []);
const characters = useStorage<
  Array<{
    name: string;
    description: string;
    gender: string;
    isMain: boolean;
    avatarPath?: string;
  }>
>('mg_characters', [
  {
    name: '主角',
    description: '故事的核心人物',
    gender: '男',
    isMain: true,
  },
]);
/** GLM 的默认请求地址（用于判定“是否被修改”） */
const DEFAULT_GLM_BASE_URL =
  'https://open.bigmodel.cn/api/paas/v4/chat/completions';
/** GLM 的默认模型（用于判定“是否被修改”） */
const DEFAULT_GLM_MODEL = 'glm-4.6v-flash';

const glmBaseUrl = useStorage('mg_glm_base_url', DEFAULT_GLM_BASE_URL);
const glmModel = useStorage('mg_glm_model', DEFAULT_GLM_MODEL);

/** 首页“剧情类型”可选项（与首页保持一致） */
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
  '爽文',
];

/** 自定义类型输入框的临时值 */
const customGenre = ref('');

/**
 * 数据安全锁：当用户自行修改模型配置时，禁用分享与设计功能。
 */
const securityLocked = computed(() => {
  const baseUrlTouched = glmBaseUrl.value.trim() !== DEFAULT_GLM_BASE_URL;
  const modelTouched = glmModel.value.trim() !== DEFAULT_GLM_MODEL;
  return baseUrlTouched || modelTouched;
});

const isLoading = ref(false);
const loadError = ref('');

const playEntry = ref<PlayEntry>('owner');
const isOwner = ref(false);
const accessError = ref('');

const toast = ref<{ text: string; kind: 'info' | 'success' | 'error' } | null>(
  null,
);
let toastTimer: ReturnType<typeof setTimeout> | null = null;

type ConfirmModalState = {
  title: string;
  message: string;
  confirmText: string;
  cancelText: string;
  kind: 'danger' | 'info';
  onConfirm: () => void;
};

const confirmModal = ref<ConfirmModalState | null>(null);

/**
 * 打开确认弹窗（用于替代浏览器 confirm）。
 */
const openConfirm = (state: ConfirmModalState) => {
  confirmModal.value = state;
};

const closeConfirm = () => {
  confirmModal.value = null;
};

/**
 * 执行确认弹窗的确认回调。
 */
const runConfirm = () => {
  const action = confirmModal.value?.onConfirm;
  confirmModal.value = null;
  action?.();
};

/**
 * 统一展示提示信息。
 */
const showToast = (text: string, kind: 'info' | 'success' | 'error') => {
  toast.value = { text, kind };
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.value = null;
  }, 2200);
};

onUnmounted(() => {
  if (toastTimer) clearTimeout(toastTimer);
});

const draft = ref<MovieTemplate | null>(null);
const dirty = ref(false);
const isSaving = ref(false);

const showJsonModal = ref(false);

const showImportModal = ref(false);
const importTab = ref<'paste' | 'file'>('paste');
const importText = ref('');
const importError = ref('');
const isImportApplying = ref(false);

const pendingTemplateSource = ref<string | null>(null);

/**
 * 打开导入弹窗。
 */
const openImportModal = () => {
  importTab.value = 'paste';
  importText.value = '';
  importError.value = '';
  isImportApplying.value = false;
  showImportModal.value = true;
};

/**
 * 关闭导入弹窗。
 */
const closeImportModal = () => {
  showImportModal.value = false;
};

/**
 * 解析导入 JSON，返回 MovieTemplate。
 */
const parseImportData = (): MovieTemplate | null => {
  importError.value = '';
  try {
    const raw = importText.value.trim();
    if (!raw) {
      importError.value = '请粘贴或上传 JSON';
      return null;
    }

    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== 'object') {
      importError.value = 'JSON 解析失败，请检查格式';
      return null;
    }

    const obj = parsed as Record<string, unknown>;

    const fromProcessedResponse =
      (obj.processed_response && typeof obj.processed_response === 'object'
        ? obj.processed_response
        : null) ??
      (obj.processedResponse && typeof obj.processedResponse === 'object'
        ? obj.processedResponse
        : null);

    const fromGenerateResponse =
      obj.template && typeof obj.template === 'object' ? obj.template : null;

    const dataRaw = (fromProcessedResponse ??
      fromGenerateResponse ??
      obj) as unknown;
    if (!dataRaw || typeof dataRaw !== 'object') {
      importError.value = 'JSON 解析失败，请检查格式';
      return null;
    }

    const record = dataRaw as Record<string, unknown>;

    const nodes = record.nodes;
    if (!nodes || typeof nodes !== 'object') {
      importError.value = 'JSON 缺少 nodes';
      return null;
    }

    const createId = () => {
      try {
        return crypto.randomUUID();
      } catch {
        return `p_${Date.now()}_${Math.random().toString(16).slice(2)}`;
      }
    };

    const metaRaw = record.meta;
    const meta =
      metaRaw && typeof metaRaw === 'object'
        ? (metaRaw as Record<string, unknown>)
        : ({} as Record<string, unknown>);

    const normalized: MovieTemplate = {
      projectId:
        typeof record.projectId === 'string' && record.projectId.trim()
          ? record.projectId.trim()
          : createId(),
      title:
        typeof record.title === 'string' && record.title.trim()
          ? record.title
          : theme.value.trim(),
      version:
        typeof record.version === 'string' && record.version.trim()
          ? record.version.trim()
          : '1.0.0',
      owner:
        typeof record.owner === 'string' && record.owner.trim()
          ? record.owner.trim()
          : 'User',
      meta: {
        logline:
          typeof meta.logline === 'string' && meta.logline.trim()
            ? meta.logline
            : theme.value.trim(),
        synopsis:
          typeof meta.synopsis === 'string' && meta.synopsis.trim()
            ? meta.synopsis
            : synopsis.value.trim(),
        targetRuntimeMinutes:
          typeof meta.targetRuntimeMinutes === 'number' &&
          Number.isFinite(meta.targetRuntimeMinutes)
            ? meta.targetRuntimeMinutes
            : 0,
        genre:
          typeof meta.genre === 'string' && meta.genre.trim()
            ? meta.genre
            : (selectedGenres.value || []).join(' / '),
        language:
          typeof meta.language === 'string' && meta.language.trim()
            ? meta.language
            : String(navigator.language || '').trim(),
      },
      backgroundImageBase64:
        typeof record.backgroundImageBase64 === 'string'
          ? record.backgroundImageBase64
          : undefined,
      nodes: record.nodes as MovieTemplate['nodes'],
      endings:
        record.endings && typeof record.endings === 'object'
          ? (record.endings as MovieTemplate['endings'])
          : {},
      characters:
        record.characters && typeof record.characters === 'object'
          ? (record.characters as MovieTemplate['characters'])
          : {},
      provenance:
        record.provenance && typeof record.provenance === 'object'
          ? (record.provenance as MovieTemplate['provenance'])
          : {
              createdBy: 'import',
              createdAt: new Date().toISOString(),
            },
    };

    const reqId = obj.id;
    if (
      obj.template &&
      typeof obj.template === 'object' &&
      typeof reqId === 'string' &&
      reqId.trim()
    ) {
      normalized.requestId = reqId.trim();
    } else if (
      typeof record.requestId === 'string' &&
      record.requestId.trim()
    ) {
      normalized.requestId = record.requestId.trim();
    }

    if (
      (!normalized.characters ||
        Object.keys(normalized.characters).length === 0) &&
      Array.isArray(characters.value)
    ) {
      const fallbackChars = characters.value
        .map((c) => {
          const name = String(c.name || '').trim();
          if (!name) return null;
          return {
            id: name,
            name,
            gender: String(c.gender || '其他').trim() || '其他',
            age: 0,
            role: String(c.description || '').trim(),
            background: '',
            avatarPath: c.avatarPath || undefined,
          };
        })
        .filter(Boolean) as MovieTemplate['characters'][string][];

      const map: MovieTemplate['characters'] = {};
      for (const ch of fallbackChars) map[ch.id] = ch;
      if (Object.keys(map).length > 0) normalized.characters = map;
    }

    if (
      !normalized.characters ||
      Object.keys(normalized.characters).length === 0
    ) {
      normalized.characters = {
        主角: {
          id: '主角',
          name: '主角',
          gender: '其他',
          age: 0,
          role: '',
          background: '',
        },
      };
    }

    return normalized;
  } catch {
    importError.value = 'JSON 解析失败，请检查格式';
    return null;
  }
};

/**
 * 读取 JSON 文件，并写入导入文本框。
 */
const onImportFile = (event: Event) => {
  const input = event.target as HTMLInputElement;
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

/**
 * 将导入的模板覆盖到当前设计器数据（保留当前 requestId 归属）。
 */
const overwriteWithImportedTemplate = async (data: MovieTemplate) => {
  const imported = cloneJson(data);

  if (playEntry.value === 'import') {
    imported.requestId = '';
  } else {
    const currentRequestId = String(draft.value?.requestId || '').trim();
    if (currentRequestId) imported.requestId = currentRequestId;
  }

  clearRunState();
  await persistActiveGameData(imported);
  draft.value = cloneJson(imported);
  dirty.value = true;

  if (
    playEntry.value === 'owner' &&
    String(imported.requestId || '').trim() !== ''
  ) {
    pendingTemplateSource.value = 'import';
  }

  hydrateLocalInputsFromDraft(imported);
};

/**
 * 导入并覆盖（仅本地）。
 */
const confirmImportOverwriteLocal = async () => {
  if (isImportApplying.value) return;
  const data = parseImportData();
  if (!data) return;

  isImportApplying.value = true;
  try {
    await overwriteWithImportedTemplate(data);
    closeImportModal();
    showToast('已导入并覆盖当前剧情', 'success');
  } finally {
    isImportApplying.value = false;
  }
};

/**
 * 导入、覆盖并保存到数据库（仅创建者模式）。
 */
const confirmImportOverwriteSave = async () => {
  if (isImportApplying.value) return;

  if (securityLocked.value) {
    importError.value =
      '检测到本地模型配置已被修改（Base URL / Model）。为确保数据安全，已禁用设计与分享功能。请先在首页设置中恢复默认配置。';
    return;
  }

  if (!canEdit.value || playEntry.value !== 'owner') {
    importError.value = '当前无保存权限，无法覆盖并保存';
    return;
  }

  const requestId = String(draft.value?.requestId || '').trim();
  if (!requestId) {
    importError.value = '当前没有可保存的数据库记录，请先生成或从记录进入';
    return;
  }

  const data = parseImportData();
  if (!data) return;

  isImportApplying.value = true;
  importError.value = '';
  try {
    await overwriteWithImportedTemplate(data);

    const current = draft.value;
    if (!current) {
      importError.value = '当前没有可编辑的数据，请先生成或导入剧情。';
      return;
    }

    const saved = await updateGameTemplate(requestId, current, 'import');
    pendingTemplateSource.value = null;
    draft.value = cloneJson(saved);
    await persistActiveGameData(saved);
    hydrateLocalInputsFromDraft(saved);
    dirty.value = false;
    closeImportModal();
    showToast('已覆盖并保存到数据库', 'success');
  } catch (e: unknown) {
    if (e instanceof ApiError) {
      importError.value = e.message || '覆盖保存失败';
      return;
    }
    importError.value = e instanceof Error ? e.message : '覆盖保存失败';
  } finally {
    isImportApplying.value = false;
  }
};

const isShared = ref(false);
const shareLoading = ref(false);
const showShareModal = ref(false);
const sharedRecordId = ref<string | null>(null);
const sharedAt = ref<string | null>(null);
const recordIds = useStorage<string[]>('mg_record_ids', []);

/**
 * 仅创建者（且存在 requestId）才可分享。
 */
const canShare = computed(() => {
  const requestId = String(draft.value?.requestId || '').trim();
  return playEntry.value === 'owner' && canEdit.value && requestId !== '';
});

const shareLink = computed(() => {
  if (!canShare.value) return '';
  const requestId = String(draft.value?.requestId || '').trim();
  if (!requestId) return '';
  return `${window.location.origin}/play/${requestId}`;
});

/**
 * 读取分享元信息，避免非创建人在设计页看到分享入口。
 */
const refreshShareMeta = async () => {
  if (playEntry.value === 'import') {
    isShared.value = false;
    sharedRecordId.value = null;
    sharedAt.value = null;
    return;
  }

  const requestId = String(draft.value?.requestId || '').trim();
  if (!requestId) {
    isShared.value = false;
    sharedRecordId.value = null;
    sharedAt.value = null;
    return;
  }

  try {
    const meta = await getSharedRecordMeta(requestId);
    isOwner.value = meta.isOwner;
    isShared.value = meta.shared;
    sharedRecordId.value = meta.sharedRecordId;
    sharedAt.value = meta.sharedAt;
  } catch {
    isShared.value = false;
    sharedRecordId.value = null;
    sharedAt.value = null;
  }
};

/**
 * 切换分享状态：分享成功时弹出链接弹窗。
 */
const handleShare = async () => {
  if (playEntry.value === 'import') {
    showToast('手动导入的剧情不支持在线分享', 'error');
    return;
  }

  if (securityLocked.value) {
    showToast(
      '检测到本地模型配置已被修改（数据安全），已禁用分享功能',
      'error',
    );
    return;
  }

  const requestId = String(draft.value?.requestId || '').trim();
  if (!requestId) {
    showToast('此数据不支持在线分享', 'error');
    return;
  }

  if (!isOwner.value) {
    showToast('只有创建人才可以分享此剧情', 'error');
    return;
  }

  shareLoading.value = true;
  try {
    const nextState = !isShared.value;
    const resp = await shareGame(requestId, nextState);
    isShared.value = nextState;
    await refreshShareMeta();

    if (nextState) {
      if (resp.sharedRecordId) {
        sharedRecordId.value = resp.sharedRecordId;
        if (!recordIds.value.includes(resp.sharedRecordId)) {
          recordIds.value = [...recordIds.value, resp.sharedRecordId];
        }
      }
      showShareModal.value = true;
    } else {
      showToast('已取消分享', 'success');
      showShareModal.value = false;
    }
  } catch (e: unknown) {
    console.error('Share failed:', e);
    const msg = e instanceof Error ? e.message : '分享状态更新失败';
    showToast(msg, 'error');
  } finally {
    shareLoading.value = false;
  }
};

/**
 * 复制分享链接。
 */
const copyShareLink = async () => {
  try {
    await navigator.clipboard.writeText(shareLink.value);
    showToast('链接已复制到剪贴板', 'success');
  } catch {
    showToast('复制失败，请重试', 'error');
  }
};

const exportData = computed(() => {
  return draft.value ?? gameData.value ?? null;
});

const jsonContent = computed(() => {
  const data = exportData.value;
  if (!data) return '{}';

  const cloned = cloneJson(data);
  delete (cloned as unknown as { requestId?: string }).requestId;

  const endings = (cloned as unknown as { endings?: unknown }).endings;
  if (!endings || typeof endings !== 'object') {
    (cloned as unknown as { endings: Record<string, unknown> }).endings = {};
  }

  return JSON.stringify(cloned, null, 2);
});

const copyJson = async () => {
  try {
    await navigator.clipboard.writeText(jsonContent.value);
    showToast('JSON 已复制到剪贴板', 'success');
  } catch {
    showToast('复制失败，请重试', 'error');
  }
};

const downloadJson = () => {
  const data = exportData.value;
  if (!data) return;
  const blob = new Blob([jsonContent.value], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `movie-game-${data?.title || 'export'}-${Date.now()}.json`;
  a.click();
  URL.revokeObjectURL(url);
};

const fileToDataUrl = (file: File) => {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result || ''));
    reader.onerror = () => reject(new Error('read_failed'));
    reader.readAsDataURL(file);
  });
};

const syncAvatarToDraftByName = (
  nameRaw: string,
  avatarPath: string | undefined,
) => {
  const name = String(nameRaw || '').trim();
  if (!name) return;

  const d = draft.value;
  if (!d?.characters) return;

  let changed = false;
  for (const c of Object.values(d.characters)) {
    if (String(c.name || '').trim() !== name) continue;
    if (c.avatarPath === avatarPath) continue;
    c.avatarPath = avatarPath;
    changed = true;
  }

  if (changed) dirty.value = true;
};

const setCharacterAvatar = async (idx: number, e: Event) => {
  const input = e.target as HTMLInputElement | null;
  const file = input?.files?.[0];
  if (!file) return;
  try {
    const dataUrl = await fileToDataUrl(file);
    const next = [...characters.value];
    const cur = next[idx];
    if (!cur) return;
    next[idx] = { ...cur, avatarPath: dataUrl };
    characters.value = next;
    syncAvatarToDraftByName(cur.name, dataUrl);
    showToast('头像已更新', 'success');
  } catch {
    showToast('头像读取失败', 'error');
  } finally {
    if (input) input.value = '';
  }
};

const clearCharacterAvatar = (idx: number) => {
  const next = [...characters.value];
  const cur = next[idx];
  if (!cur) return;
  next[idx] = { ...cur, avatarPath: undefined };
  characters.value = next;
  syncAvatarToDraftByName(cur.name, undefined);
  showToast('头像已清除', 'info');
};

/**
 * 将模板的 genre 字段解析成可用于标签展示的列表。
 */
const parseGenreTags = (genreRaw: string): string[] => {
  const genre = String(genreRaw || '').trim();
  if (!genre) return [];
  return Array.from(
    new Set(
      genre
        .split(/\s*(?:\/|\||,|，|、|;|；)\s*/g)
        .map((x) => x.trim())
        .filter(Boolean),
    ),
  );
};

/**
 * 用当前剧情模板数据覆盖设计页的“首页输入区”展示值，避免读到旧的本地向导数据。
 */
const hydrateLocalInputsFromDraft = (d: MovieTemplate) => {
  theme.value = String(d.meta?.logline || d.title || '').trim();
  synopsis.value = String(d.meta?.synopsis || '').trim();

  const nextGenres = parseGenreTags(String(d.meta?.genre || '').trim());
  selectedGenres.value = nextGenres;

  const existingByName = new Map(
    characters.value
      .map((c) => [String(c.name || '').trim(), c] as const)
      .filter(([name]) => Boolean(name)),
  );

  const templateChars = Object.values(d.characters ?? {}).slice();
  templateChars.sort((a, b) =>
    String(a.name || '').localeCompare(String(b.name || ''), 'zh-Hans-CN'),
  );

  characters.value = templateChars.map((c) => {
    const name = String(c.name || '').trim();
    const existing = existingByName.get(name);

    const derivedDesc = [
      String(c.role || '').trim(),
      String(c.background || '').trim(),
    ]
      .filter(Boolean)
      .join('：');

    return {
      name,
      description: existing?.description ?? derivedDesc,
      gender: String(existing?.gender || c.gender || '其他'),
      isMain: Boolean(existing?.isMain ?? false),
      avatarPath: existing?.avatarPath ?? c.avatarPath,
    };
  });
};

const nodeModalOpen = ref(false);
const editingNodeId = ref<string | null>(null);

const endingModalOpen = ref(false);
const editingEndingKey = ref<string | null>(null);

const editingNode = computed(() => {
  const d = draft.value;
  const id = editingNodeId.value;
  if (!d || !id) return null;
  return d.nodes?.[id] ?? null;
});

const editingEnding = computed(() => {
  const d = draft.value;
  const key = editingEndingKey.value;
  if (!d || !key) return null;
  return d.endings?.[key] ?? null;
});

/**
 * 当 start 节点存在但没有任何选项时，将节点 1 视为起始节点。
 */
const fallbackStartToOne = computed(() => {
  const nodes = draft.value?.nodes;
  if (!nodes?.start) return false;
  if (!nodes['1']) return false;
  const choices = (nodes.start as StoryNode).choices;
  return !Array.isArray(choices) || choices.length === 0;
});

const startNodeId = computed(() => {
  const nodes = draft.value?.nodes;
  if (!nodes) return '';

  if (fallbackStartToOne.value) return '1';
  if (nodes.start) return 'start';
  if (nodes.root) return 'root';
  if (nodes['1']) return '1';
  return Object.keys(nodes)[0] || '';
});

const cloneJson = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;

const readPlayEntry = (): PlayEntry => {
  const raw = String(sessionStorage.getItem('mg_play_entry') || '').trim();
  if (raw === 'shared') return 'shared';
  if (raw === 'import') return 'import';
  return 'owner';
};

const clearRunState = () => {
  localStorage.removeItem('mg_current_node');
  localStorage.removeItem('mg_player_state');
  localStorage.removeItem('mg_history_stack');
  localStorage.removeItem('mg_ending');
};

const generateNextNodeId = (nodes: Record<string, StoryNode>) => {
  const keys = Object.keys(nodes);
  const numeric = keys
    .map((k) => Number.parseInt(k, 10))
    .filter((n) => Number.isFinite(n));
  if (numeric.length > 0) {
    const next = Math.max(...numeric) + 1;
    const id = String(next);
    if (!nodes[id]) return id;
  }

  let i = keys.length + 1;
  while (nodes[`node_${i}`]) i += 1;
  return `node_${i}`;
};

const replaceNextNodeId = (
  nodes: Record<string, StoryNode>,
  oldKey: string,
  newKey: string,
) => {
  for (const n of Object.values(nodes)) {
    for (const c of n.choices || []) {
      if (c.nextNodeId === oldKey) c.nextNodeId = newKey;
    }
  }
};

const treeGraph = computed(() => {
  const nodes: Record<string, StoryNode> = draft.value?.nodes ?? {};
  const endings = draft.value?.endings ?? {};
  const root = startNodeId.value;
  if (!root || !nodes[root]) {
    return {
      nodes: [] as TreeNodeVM[],
      edges: [] as EdgeVM[],
      view: { w: 1000, h: 700 },
      parent: new Map<string, string>(),
    };
  }

  const knownEndingKeys = new Set(Object.keys(endings));
  const children = new Map<string, { to: string; label?: string }[]>();

  for (const [id, n] of Object.entries(nodes)) {
    const list: { to: string; label?: string }[] = [];

    const seenTargets = new Set<string>();
    for (const c of n.choices || []) {
      const to = (c.nextNodeId || '').trim();
      if (!to) continue;
      if (to === 'END') continue;
      if (seenTargets.has(to)) continue;
      seenTargets.add(to);

      if (nodes[to]) list.push({ to, label: c.text });
      else if (knownEndingKeys.has(to)) list.push({ to, label: c.text });
    }
    children.set(id, list);
  }

  const visited = new Set<string>();
  const parent = new Map<string, string>();
  const depth = new Map<string, number>();
  const q: string[] = [root];
  visited.add(root);
  depth.set(root, 0);

  const edges: EdgeVM[] = [];

  while (q.length > 0) {
    const cur = q.shift() as string;
    const d = depth.get(cur) ?? 0;
    const next = children.get(cur) ?? [];

    for (const e of next) {
      edges.push({ from: cur, to: e.to, label: e.label });
      if (visited.has(e.to)) continue;
      visited.add(e.to);
      parent.set(e.to, cur);
      depth.set(e.to, d + 1);
      q.push(e.to);
    }
  }

  if (fallbackStartToOne.value && nodes.start) {
    visited.add('start');
    depth.set('start', 0);
  }

  const byDepth = new Map<number, string[]>();
  for (const id of visited) {
    const d = depth.get(id) ?? 0;
    if (!byDepth.has(d)) byDepth.set(d, []);
    byDepth.get(d)?.push(id);
  }

  const maxDepth = Math.max(...Array.from(byDepth.keys()));

  const layers: string[][] = [];
  for (let i = 0; i <= maxDepth; i++) layers.push([]);

  const placed = new Set<string>();

  const rootLayer = byDepth.get(0) || [];
  rootLayer.sort();
  layers[0] = rootLayer;
  rootLayer.forEach((id) => {
    placed.add(id);
  });

  for (let d = 0; d < maxDepth; d++) {
    const currentLayer = layers[d] ?? [];
    const nextLayerCandidates: string[] = [];

    for (const pid of currentLayer) {
      const kids = children.get(pid) || [];
      kids.sort((a, b) => (a.label || '').localeCompare(b.label || ''));

      for (const k of kids) {
        if (depth.get(k.to) === d + 1 && !placed.has(k.to)) {
          nextLayerCandidates.push(k.to);
          placed.add(k.to);
        }
      }
    }

    const originalNextLayer = byDepth.get(d + 1) || [];
    originalNextLayer.sort();
    for (const id of originalNextLayer) {
      if (!placed.has(id)) {
        nextLayerCandidates.push(id);
        placed.add(id);
      }
    }

    layers[d + 1] = nextLayerCandidates;
  }

  const xStep = 260;
  const yStep = 130;
  const padX = 50;
  const padY = 50;
  const cardW = 200;
  const cardH = 100;

  let maxRow = 0;
  for (const layer of layers) {
    if (!layer) continue;
    maxRow = Math.max(maxRow, layer.length);
  }

  const totalH = Math.max(1, maxRow) * yStep;
  const totalW = padX * 2 + (maxDepth + 1) * xStep;

  const pos = new Map<string, { x: number; y: number }>();

  for (let d = 0; d <= maxDepth; d++) {
    const layer = layers[d] ?? [];
    const layerH = layer.length * yStep;
    const startY = padY + (totalH - layerH) / 2;

    for (let i = 0; i < layer.length; i++) {
      const id = layer[i];
      if (!id) continue;
      pos.set(id, {
        x: padX + d * xStep,
        y: startY + i * yStep,
      });
    }
  }

  const nodeVMs: TreeNodeVM[] = [];
  for (const id of visited) {
    const p = pos.get(id) ?? { x: padX, y: padY };
    const isEnding = knownEndingKeys.has(id);
    nodeVMs.push({
      id,
      kind: isEnding ? 'ending' : 'story',
      label: isEnding ? `END:${id}` : id,
      depth: depth.get(id) ?? 0,
      x: p.x,
      y: p.y,
      w: cardW,
      h: cardH,
    });
  }

  return {
    nodes: nodeVMs,
    edges,
    view: { w: totalW, h: totalH + padY * 2 },
    parent,
  };
});

const reachableStoryNodeIds = computed(() => {
  const out = new Set<string>();
  for (const n of treeGraph.value.nodes) {
    if (n.kind !== 'story') continue;
    out.add(n.id);
  }
  return out;
});

const orphanNodeIds = computed(() => {
  const d = draft.value;
  if (!d) return [] as string[];
  const nodes = d.nodes || {};
  const root = startNodeId.value;

  const incoming = new Map<string, number>();
  for (const id of Object.keys(nodes)) incoming.set(id, 0);

  for (const n of Object.values(nodes)) {
    for (const c of n.choices || []) {
      const to = String(c.nextNodeId || '').trim();
      if (!to) continue;
      if (!nodes[to]) continue;
      incoming.set(to, (incoming.get(to) ?? 0) + 1);
    }
  }

  const reachable = reachableStoryNodeIds.value;
  const forceOrphanStart = fallbackStartToOne.value && Boolean(nodes.start);

  return Object.keys(nodes)
    .filter((id) => {
      if (id === root) return false;
      if (forceOrphanStart && id === 'start') return true;
      const inCount = incoming.get(id) ?? 0;
      return inCount === 0 || !reachable.has(id);
    })
    .sort();
});

const orphanNodeReasonMap = computed(() => {
  const d = draft.value;
  if (!d) return new Map<string, string>();

  const nodes = d.nodes || {};
  const root = startNodeId.value;

  const incoming = new Map<string, number>();
  for (const id of Object.keys(nodes)) incoming.set(id, 0);

  for (const n of Object.values(nodes)) {
    for (const c of n.choices || []) {
      const to = String(c.nextNodeId || '').trim();
      if (!to) continue;
      if (!nodes[to]) continue;
      incoming.set(to, (incoming.get(to) ?? 0) + 1);
    }
  }

  const reachable = reachableStoryNodeIds.value;
  const forceOrphanStart = fallbackStartToOne.value && Boolean(nodes.start);

  const out = new Map<string, string>();
  for (const id of orphanNodeIds.value) {
    if (id === root) continue;
    if (forceOrphanStart && id === 'start') {
      out.set(id, 'start 无选项，已使用 1 作为起点');
      continue;
    }
    const inCount = incoming.get(id) ?? 0;
    const isReachable = reachable.has(id);
    if (inCount === 0 && !isReachable) out.set(id, '不可达且无入边');
    else if (!isReachable) out.set(id, '从 start 不可达');
    else out.set(id, '无入边');
  }
  return out;
});

const selectedId = ref<string>('');
const hoveredId = ref<string>('');
const treeWrapEl = ref<HTMLDivElement | null>(null);
const pan = ref({ x: 0, y: 0 });
const zoom = ref(0.9);
const dragging = ref<null | {
  x: number;
  y: number;
  panX: number;
  panY: number;
}>(null);
const isDragging = ref(false);
const dragStart = ref({ x: 0, y: 0 });

const highlighted = computed(() => {
  const focus = selectedId.value;
  const parent = treeGraph.value.parent;
  if (!focus || !parent) return new Set<string>();
  const out = new Set<string>();
  let cur: string | undefined = focus;
  let guard = 0;
  while (cur && guard < 2000) {
    out.add(cur);
    cur = parent.get(cur);
    guard += 1;
  }
  return out;
});

const clamp = (v: number, min: number, max: number) =>
  Math.max(min, Math.min(max, v));

const resetView = () => {
  zoom.value = 0.9;
  pan.value = { x: 0, y: 0 };
};

const fitTree = async () => {
  await nextTick();
  const el = treeWrapEl.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  const view = treeGraph.value.view;

  const availableW = rect.width - 40;
  const availableH = rect.height - 40;

  let scale = Math.min(availableW / view.w, availableH / view.h);

  if (scale < 0.6) {
    scale = 0.8;
    zoom.value = scale;
    pan.value = {
      x: 40,
      y: (rect.height - view.h * scale) / 2,
    };
  } else {
    scale = clamp(scale, 0.5, 1.5);
    zoom.value = scale;
    pan.value = {
      x: (rect.width - view.w * scale) / 2,
      y: (rect.height - view.h * scale) / 2,
    };
  }
};

watch(
  () => startNodeId.value,
  () => {
    selectedId.value = '';
    fitTree();
  },
  { immediate: true },
);

const onWheel = (e: WheelEvent) => {
  e.preventDefault();
  const el = treeWrapEl.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  const mx = e.clientX - rect.left;
  const my = e.clientY - rect.top;

  const delta = -e.deltaY;
  const factor = delta > 0 ? 1.08 : 0.92;
  const prevZoom = zoom.value;
  const nextZoom = clamp(prevZoom * factor, 0.1, 4.0);

  const wx = (mx - pan.value.x) / prevZoom;
  const wy = (my - pan.value.y) / prevZoom;
  zoom.value = nextZoom;
  pan.value = {
    x: mx - wx * nextZoom,
    y: my - wy * nextZoom,
  };
};

const onPointerDown = (e: PointerEvent) => {
  dragStart.value = { x: e.clientX, y: e.clientY };
  isDragging.value = false;

  dragging.value = {
    x: e.clientX,
    y: e.clientY,
    panX: pan.value.x,
    panY: pan.value.y,
  };
};

const onPointerMove = (e: PointerEvent) => {
  if (!dragging.value) return;

  const dx = e.clientX - dragging.value.x;
  const dy = e.clientY - dragging.value.y;

  if (
    !isDragging.value &&
    (Math.abs(e.clientX - dragStart.value.x) > 5 ||
      Math.abs(e.clientY - dragStart.value.y) > 5)
  ) {
    isDragging.value = true;
  }

  pan.value = { x: dragging.value.panX + dx, y: dragging.value.panY + dy };
};

const onPointerUp = () => {
  dragging.value = null;
  setTimeout(() => {
    isDragging.value = false;
  }, 50);
};

const onNodeClick = (id: string) => {
  if (isDragging.value) return;
  selectedId.value = id;
};

const onGlobalPointerUp = () => {
  dragging.value = null;
};

onMounted(() => {
  window.addEventListener('pointerup', onGlobalPointerUp);
  window.addEventListener('pointercancel', onGlobalPointerUp);
});

onUnmounted(() => {
  window.removeEventListener('pointerup', onGlobalPointerUp);
  window.removeEventListener('pointercancel', onGlobalPointerUp);
});

/**
 * 基于模板角色表构建 id → name 的映射，用于兼容历史数据里使用 id 存储角色的情况。
 */
const characterIdToName = computed(() => {
  const templateChars = Object.values(draft.value?.characters ?? {});
  return new Map(
    templateChars
      .map(
        (c) =>
          [String(c.id || '').trim(), String(c.name || '').trim()] as const,
      )
      .filter(([id, name]) => Boolean(id && name)),
  );
});

/**
 * 将节点的角色字段统一解析为角色名称列表（去重、去空、兼容 id）。
 */
const resolveCharacterNames = (rawList: unknown): string[] => {
  const idToName = characterIdToName.value;
  const list = Array.isArray(rawList) ? rawList : [];
  const out: string[] = [];
  for (const raw of list) {
    const v = String(raw || '').trim();
    if (!v) continue;
    out.push(idToName.get(v) || v);
  }
  return Array.from(new Set(out));
};

const referencedCharacterNames = computed(() => {
  const out = new Set<string>();
  const nodes = draft.value?.nodes ?? {};
  for (const n of Object.values(nodes)) {
    for (const name of resolveCharacterNames(n.characters)) out.add(name);
  }
  return out;
});

const canRemoveCharacter = (idx: number) => {
  const name = String(characters.value[idx]?.name || '').trim();
  if (!name) return true;
  return !referencedCharacterNames.value.has(name);
};

const removeCharacter = (idx: number) => {
  const cur = characters.value[idx];
  if (!cur) return;
  const name = String(cur.name || '').trim();
  if (name && referencedCharacterNames.value.has(name)) {
    showToast(`角色“${name}”正在被节点引用，无法删除`, 'error');
    return;
  }
  characters.value = characters.value.filter((_, i) => i !== idx);
  showToast('已移除角色', 'info');
};

const selectedNodeInfo = computed(() => {
  const id = selectedId.value;
  if (!id) return null;
  const nodes = draft.value?.nodes ?? {};
  const endings = draft.value?.endings ?? {};
  if (endings[id]) {
    return {
      id,
      kind: 'ending' as const,
      description: endings[id].description,
      type: endings[id].type,
    };
  }

  const n = nodes[id];
  if (!n) return null;
  return {
    id,
    kind: 'story' as const,
    content:
      typeof n.content === 'string'
        ? n.content
        : (n.content as unknown as { text?: string } | undefined)?.text || '',
    characters: resolveCharacterNames(n.characters),
    choices: (n.choices || []).map((c) => ({ text: c.text, to: c.nextNodeId })),
  };
});

const endingKeys = computed(() => {
  return Object.keys(draft.value?.endings ?? {}).sort();
});

const canEdit = computed(() => {
  if (securityLocked.value) return false;
  if (playEntry.value === 'import') return true;
  if (playEntry.value === 'owner') return isOwner.value;
  return false;
});

const goBack = () => {
  router.back();
};

const goHome = () => {
  router.push('/');
};

const goPlay = async () => {
  if (!draft.value) return;
  if (!canEdit.value) return;
  await applyDraft({ forceDbSave: true });
  clearRunState();
  await nextTick();
  router.push('/game');
};

const openNode = (id: string) => {
  if (!canEdit.value) return;

  const d = draft.value;
  const node = d?.nodes?.[id];
  if (node) {
    if (String((node as { id?: unknown }).id || '').trim() === '') {
      (node as { id?: string }).id = id;
      dirty.value = true;
    }

    if (!Array.isArray(node.choices)) {
      node.choices = [];
      dirty.value = true;
    }
  }

  editingNodeId.value = id;
  nodeModalOpen.value = true;
};

const closeNodeModal = () => {
  nodeModalOpen.value = false;
  editingNodeId.value = null;
};

const openEnding = (key: string) => {
  if (!canEdit.value) return;
  editingEndingKey.value = key;
  endingModalOpen.value = true;
};

const closeEndingModal = () => {
  endingModalOpen.value = false;
  editingEndingKey.value = null;
};

const addNodeAtRoot = () => {
  if (!draft.value) return;
  const nodes = draft.value.nodes || {};
  const id = generateNextNodeId(nodes);
  nodes[id] = { id, content: '', characters: [], choices: [] };
  draft.value.nodes = nodes;
  dirty.value = true;
  openNode(id);
};

const deleteNode = (id: string) => {
  if (!draft.value) return;
  if (id === 'start') {
    showToast('start 节点不可删除', 'error');
    return;
  }

  openConfirm({
    title: '删除节点',
    message: `确定删除节点 "${id}" 吗？引用到它的选项会被移除。`,
    confirmText: '删除',
    cancelText: '取消',
    kind: 'danger',
    onConfirm: () => {
      const cur = draft.value;
      if (!cur) return;

      const nodes = cur.nodes || {};
      if (!nodes[id]) return;

      delete nodes[id];
      for (const n of Object.values(nodes)) {
        n.choices = (n.choices || []).filter((c) => c.nextNodeId !== id);
      }

      const endings = cur.endings || {};
      if (endings[id]) {
        delete endings[id];
        cur.endings = endings;
      }

      cur.nodes = nodes;
      dirty.value = true;

      if (editingNodeId.value === id) closeNodeModal();
    },
  });
};

const addChoice = (nodeId: string) => {
  if (!draft.value) return;
  const n = draft.value.nodes?.[nodeId];
  if (!n) return;
  n.choices = n.choices || [];
  n.choices.push({ text: '新的选择', nextNodeId: '' });
  dirty.value = true;
};

const removeChoice = (nodeId: string, idx: number) => {
  if (!draft.value) return;
  const n = draft.value.nodes?.[nodeId];
  if (!n) return;
  n.choices = (n.choices || []).filter((_, i) => i !== idx);
  dirty.value = true;
};

/**
 * 节点编辑弹窗内可选择的角色名称（来源于首页“角色阵容”，并兼容节点历史残留）。
 */
const nodeCharacterOptions = computed(() => {
  const templateChars = Object.values(draft.value?.characters ?? {});

  const fromTemplate = templateChars
    .map((c) => String(c.name || '').trim())
    .filter(Boolean);

  const fromInput = characters.value
    .map((c) => String(c.name || '').trim())
    .filter(Boolean);

  const fromNode = resolveCharacterNames(editingNode.value?.characters);

  return Array.from(new Set([...fromTemplate, ...fromInput, ...fromNode])).sort(
    (a, b) => a.localeCompare(b, 'zh-Hans-CN'),
  );
});

const protagonistName = computed(() => {
  const fromInput = characters.value.find((c) => Boolean(c?.isMain));
  const n = String(fromInput?.name || '').trim();
  if (n) return n;

  const templateChars = Object.values(draft.value?.characters ?? {});
  if (templateChars.length === 0) return '';

  const scored = templateChars
    .map((c) => {
      const key = String(c.id || '').toLowerCase();
      const name = String(c.name || '').trim();
      const role = String(c.role || '').toLowerCase();
      let score = 0;
      if (
        key.includes('player') ||
        key.includes('protagonist') ||
        key.includes('main')
      )
        score += 5;
      if (
        role.includes('protagonist') ||
        role.includes('player') ||
        role.includes('main')
      )
        score += 6;
      if (name === '我' || name.includes('主角')) score += 7;
      return { score, name };
    })
    .sort((a, b) => b.score - a.score);

  return scored[0]?.name || '';
});

const affinityTargetOptions = computed(() => {
  const p = protagonistName.value;
  const present = resolveCharacterNames(editingNode.value?.characters);
  return present
    .filter((x) => x && x !== p)
    .sort((a, b) => a.localeCompare(b, 'zh-Hans-CN'));
});

const editingNodeCharacterNameSet = computed(() => {
  return new Set(resolveCharacterNames(editingNode.value?.characters));
});

/**
 * 切换当前正在编辑节点的角色（只能从候选列表中选择）。
 */
const toggleEditingNodeCharacter = (nameRaw: string) => {
  const raw = String(nameRaw || '').trim();
  const n = editingNode.value;
  if (!raw) return;
  if (!n) return;

  const idToName = characterIdToName.value;
  const resolved = idToName.get(raw) || raw;

  n.characters = resolveCharacterNames(n.characters);

  if (n.characters.includes(resolved)) {
    n.characters = n.characters.filter((x) => x !== resolved);
  } else {
    n.characters.push(resolved);
  }
  dirty.value = true;
};

const renameNodeIfNeeded = (oldId: string, newIdRaw: string) => {
  if (!draft.value) return;
  const newId = String(newIdRaw || '').trim();
  if (!newId || newId === oldId) return;

  if (oldId === 'start') {
    showToast('start 节点不支持改名', 'error');
    return;
  }

  const nodes = draft.value.nodes || {};
  if (nodes[newId]) {
    showToast('新节点 ID 已存在', 'error');
    return;
  }

  const node = nodes[oldId];
  if (!node) return;

  delete nodes[oldId];
  node.id = newId;
  nodes[newId] = node;

  replaceNextNodeId(nodes, oldId, newId);

  const endings = draft.value.endings || {};
  const oldEnding = endings[oldId];
  if (oldEnding) {
    endings[newId] = oldEnding;
    delete endings[oldId];
  }

  for (const e of Object.values(endings)) {
    if (e.nodeId === oldId) e.nodeId = newId;
  }

  draft.value.nodes = nodes;
  draft.value.endings = endings;

  editingNodeId.value = newId;
  dirty.value = true;
};

const addEnding = () => {
  if (!draft.value) return;
  const endings = draft.value.endings || {};
  let i = 1;
  while (endings[`ending_${i}`]) i += 1;
  const key = `ending_${i}`;
  endings[key] = { type: 'neutral', description: '新的结局' };
  draft.value.endings = endings;
  dirty.value = true;
  openEnding(key);
};

const deleteEnding = (key: string) => {
  if (!draft.value) return;

  openConfirm({
    title: '删除结局',
    message: `确定删除结局 "${key}" 吗？引用到它的选项会变为无效。`,
    confirmText: '删除',
    cancelText: '取消',
    kind: 'danger',
    onConfirm: () => {
      const cur = draft.value;
      if (!cur) return;

      const endings = cur.endings || {};
      if (!endings[key]) return;
      delete endings[key];
      cur.endings = endings;
      dirty.value = true;

      if (editingEndingKey.value === key) closeEndingModal();
    },
  });
};

const renameEndingIfNeeded = (oldKey: string, newKeyRaw: string) => {
  if (!draft.value) return;
  const newKey = String(newKeyRaw || '').trim();
  if (!newKey || newKey === oldKey) return;

  const endings = draft.value.endings || {};
  if (endings[newKey]) {
    showToast('新结局 Key 已存在', 'error');
    return;
  }

  const v = endings[oldKey];
  if (!v) return;

  delete endings[oldKey];
  endings[newKey] = v;

  const nodes = draft.value.nodes || {};
  replaceNextNodeId(nodes, oldKey, newKey);

  draft.value.endings = endings;
  draft.value.nodes = nodes;

  editingEndingKey.value = newKey;
  dirty.value = true;
};

/**
 * 将模板写入本地存储，并强制刷新 gameData 引用，避免继续读到旧数据。
 * @param template 最新剧情模板
 */
const persistActiveGameData = async (template: MovieTemplate) => {
  const persisted = cloneJson(template);
  localStorage.setItem('mg_active_game_data', JSON.stringify(persisted));
  gameData.value = null;
  await nextTick();
  gameData.value = persisted;
  await nextTick();
};

/**
 * 将草稿保存到本地（localStorage / reactive state）。
 */
const applyDraftLocal = async () => {
  const d = draft.value;
  if (!d) return;
  await persistActiveGameData(d);
  dirty.value = false;
};

/**
 * 将本地角色输入转换为后端接口需要的角色结构（剔除 avatarPath）。
 */
const toApiCharacters = (
  list: Array<{
    name: string;
    description: string;
    gender: string;
    isMain: boolean;
    avatarPath?: string;
  }>,
): ApiCharacterInput[] => {
  return list.map(({ avatarPath: _avatarPath, ...rest }) => rest);
};

type ApplyDraftOptions = {
  /** 是否强制尝试写回数据库（用于“保存并游玩”）。 */
  forceDbSave?: boolean;
};

/**
 * 保存草稿：
 * - 导入模式：点击保存会尝试创建一条新的数据库记录（失败则回退为本地保存）；
 * - 创建者模式且存在 requestId：优先写回数据库，再同步本地。
 * @param opts 保存策略选项
 */
const applyDraft = async (opts?: ApplyDraftOptions) => {
  const d = draft.value;
  if (!d) return false;
  if (isSaving.value) return false;

  if (playEntry.value === 'import') {
    const shouldCreateRecord = canEdit.value;

    if (shouldCreateRecord) {
      if (securityLocked.value) {
        await applyDraftLocal();
        showToast('已保存到本地（数据安全锁已启用）', 'info');
        return true;
      }

      isSaving.value = true;
      try {
        const saved = await importGameTemplate({
          template: d,
          theme: theme.value.trim() || undefined,
          synopsis: synopsis.value.trim() || undefined,
          genre:
            selectedGenres.value && selectedGenres.value.length > 0
              ? selectedGenres.value
              : undefined,
          characters: toApiCharacters(characters.value),
          language: String(navigator.language || '').trim() || undefined,
        });

        sessionStorage.setItem('mg_play_entry', 'owner');
        playEntry.value = 'owner';
        isOwner.value = true;

        draft.value = cloneJson(saved);
        await persistActiveGameData(saved);
        hydrateLocalInputsFromDraft(saved);
        dirty.value = false;

        await refreshShareMeta();

        showToast('已保存到数据库', 'success');
        return true;
      } catch (e: unknown) {
        console.error(e);
        await applyDraftLocal();
        if (e instanceof ApiError) {
          showToast(e.message || '同步数据库失败', 'error');
          return false;
        }
        showToast('已保存到本地，但同步数据库失败', 'error');
        return false;
      } finally {
        isSaving.value = false;
      }
    }
  }

  const requestId = String(d.requestId || '').trim();
  const shouldDbSave =
    playEntry.value !== 'import' &&
    requestId !== '' &&
    canEdit.value &&
    (Boolean(opts?.forceDbSave) ||
      dirty.value ||
      pendingTemplateSource.value !== null);

  if (shouldDbSave) {
    isSaving.value = true;
    try {
      const saved = await updateGameTemplate(
        requestId,
        d,
        pendingTemplateSource.value || undefined,
      );
      pendingTemplateSource.value = null;
      draft.value = cloneJson(saved);
      await persistActiveGameData(saved);
      hydrateLocalInputsFromDraft(saved);
      dirty.value = false;
      showToast('已保存到数据库', 'success');
      return true;
    } catch (e: unknown) {
      console.error(e);
      await applyDraftLocal();
      if (e instanceof ApiError) {
        showToast(e.message || '同步数据库失败', 'error');
        return false;
      }
      showToast('已保存到本地，但同步数据库失败', 'error');
      return false;
    } finally {
      isSaving.value = false;
    }
  }

  await applyDraftLocal();
  // 本地保存后也同步本地输入
  if (draft.value) hydrateLocalInputsFromDraft(draft.value);

  if (playEntry.value === 'import') {
    showToast('已保存到本地', 'success');
    return true;
  }

  if (requestId === '') {
    showToast('已保存到本地（未绑定数据库记录）', 'info');
    return true;
  }

  showToast('已保存到本地', 'success');
  return true;
};

const discardDraft = () => {
  const base = gameData.value;
  if (!base) return;
  draft.value = cloneJson(base);
  dirty.value = false;
};

const ensureDraft = () => {
  if (!gameData.value) return;
  if (draft.value) return;
  draft.value = cloneJson(gameData.value);
};

const loadByRequestId = async (id: string) => {
  isLoading.value = true;
  loadError.value = '';
  accessError.value = '';
  try {
    const meta = await getSharedRecordMeta(id);
    if (!meta.isOwner) {
      accessError.value = '只有创建人才可以通过记录进入设计器。';
      return;
    }

    const data = await getSharedGame(id);
    clearRunState();
    await nextTick();
    gameData.value = data;
    await nextTick();
    draft.value = cloneJson(data);
  } catch (e: unknown) {
    console.error(e);
    loadError.value = e instanceof Error ? e.message : '加载失败';
  } finally {
    isLoading.value = false;
  }
};

const checkOwner = async () => {
  if (playEntry.value === 'import') {
    isOwner.value = false;
    return;
  }

  const requestId = draft.value?.requestId;
  if (!requestId) {
    isOwner.value = true;
    return;
  }

  try {
    const meta = await getSharedRecordMeta(requestId);
    isOwner.value = meta.isOwner;
  } catch {
    isOwner.value = false;
  }
};

watch(
  () => draft.value?.requestId,
  () => {
    if (playEntry.value === 'owner') checkOwner();
    refreshShareMeta();
  },
);

watch(
  () => canEdit.value,
  (v) => {
    if (v) {
      accessError.value = '';
      return;
    }

    if (securityLocked.value) {
      accessError.value =
        '检测到本地模型配置已被修改（Base URL / Model）。为确保数据安全，已禁用设计与分享功能。请先在首页设置中恢复默认配置。';
      return;
    }

    if (playEntry.value === 'shared') {
      accessError.value = '分享访问模式下不允许设计与编辑。';
      return;
    }

    accessError.value = '只有创建人才可以设计与编辑此剧情。';
  },
  { immediate: true },
);

onMounted(async () => {
  playEntry.value = readPlayEntry();

  if (securityLocked.value) {
    accessError.value =
      '检测到本地模型配置已被修改（Base URL / Model）。为确保数据安全，已禁用设计与分享功能。请先在首页设置中恢复默认配置。';
    return;
  }

  if (playEntry.value === 'shared') {
    accessError.value = '分享访问模式下不允许设计与编辑。';
    return;
  }

  const queryId = String(route.query.id || '').trim();
  if (queryId) {
    sessionStorage.setItem('mg_play_entry', 'owner');
    playEntry.value = 'owner';
    await loadByRequestId(queryId);
  }

  ensureDraft();

  if (draft.value) {
    hydrateLocalInputsFromDraft(draft.value);
  }

  await checkOwner();
  await refreshShareMeta();

  if (!draft.value) {
    accessError.value = '当前没有可编辑的数据，请先生成或导入剧情。';
    return;
  }
});

watch(
  () => `${draft.value?.projectId || ''}::${draft.value?.title || ''}`,
  () => {
    if (!draft.value) return;
    hydrateLocalInputsFromDraft(draft.value);
  },
);

const availableNextIds = computed(() => {
  const d = draft.value;
  if (!d) return [] as string[];
  const nodes = Object.keys(d.nodes || {}).sort();
  const endings = Object.keys(d.endings || {}).sort();
  return ['END', ...nodes, ...endings];
});

const stats = computed(() => {
  const d = draft.value;
  const nodes = d?.nodes ?? {};
  const endings = d?.endings ?? {};
  const chars = d?.characters ?? {};
  return {
    nodes: Object.keys(nodes).length,
    endings: Object.keys(endings).length,
    characters: Object.keys(chars).length,
  };
});

/**
 * 生成节点/结局跳转选项的展示文本：显示 ID + 内容摘要。
 */
const getNextIdLabel = (idRaw: string): string => {
  const id = String(idRaw || '').trim();
  if (!id) return '';
  if (id === 'END') return 'END（结束）';

  const d = draft.value;
  const ending = d?.endings?.[id];
  if (ending) {
    const desc = String(ending.description || '').trim();
    const snippet = desc ? `：${desc.slice(0, 28)}` : '';
    return `${id}（结局${snippet}）`;
  }

  const node = d?.nodes?.[id];
  if (!node) return `${id}（不存在）`;

  const text = String(node.content || '')
    .replace(/\s+/g, ' ')
    .trim();
  const snippet = text ? `：${text.slice(0, 28)}` : '';
  return `${id}${snippet}`;
};

const patchDraft = (fn: (d: MovieTemplate) => void) => {
  const d = draft.value;
  if (!d) return;
  fn(d);
  dirty.value = true;
};

/**
 * 更新当前编辑节点的内容。
 */
const updateEditingNodeContent = (nodeId: string, e: Event) => {
  const target = e.target as HTMLTextAreaElement | null;
  const value = String(target?.value || '');
  patchDraft((d) => {
    const node = d.nodes?.[nodeId];
    if (!node) return;
    node.content = value;
  });
};

const updateChoice = (nodeId: string, idx: number, patch: Partial<Choice>) => {
  const d = draft.value;
  if (!d) return;
  const n = d.nodes?.[nodeId];
  if (!n) return;
  n.choices = n.choices || [];
  const cur = n.choices[idx];
  if (!cur) return;
  n.choices[idx] = { ...cur, ...patch };
  dirty.value = true;
};
</script>

<template>
  <div class="relative min-h-screen w-full overflow-hidden bg-black text-white">
    <WavyBackground
      container-class="fixed inset-0 z-0 pointer-events-none"
      :colors="['#38bdf8', '#818cf8', '#c084fc', '#e879f9', '#22d3ee']"
      :waveWidth="120"
      :blur="26"
      speed="fast"
    />

    <div class="relative z-10 mx-auto w-full max-w-6xl px-4 md:px-6 py-8 md:py-10">
      <header class="flex items-start justify-between gap-4">
        <div class="flex items-start gap-3">
          <button
            @click="goBack"
            class="group relative inline-flex items-center justify-center w-11 h-11 rounded-2xl border border-white/10 bg-black/35 backdrop-blur-md hover:bg-black/55 transition-all shadow-[0_0_18px_rgba(168,85,247,0.18)] overflow-hidden"
            title="返回"
          >
            <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/10 to-transparent -translate-x-full group-hover:translate-x-full transition-transform duration-700"></div>
            <ArrowLeft class="w-5 h-5 text-white/80 relative z-10" />
          </button>

          <div>
            <div class="text-xs tracking-[0.28em] uppercase text-white/50 font-semibold">
              Designer
            </div>
            <h1
              class="mt-2 text-3xl md:text-5xl font-black bg-gradient-to-r from-purple-200 via-fuchsia-300 to-cyan-200 bg-clip-text text-transparent"
            >
              剧情设计器
            </h1>
            <p class="mt-2 text-sm text-white/55 max-w-2xl leading-relaxed">
              可视化编辑本地创作输入与剧情节点结构。分享访问模式下禁止编辑，确保数据安全。
            </p>
          </div>
        </div>

        <div class="flex flex-col md:flex-row gap-2 md:gap-3 items-stretch md:items-center">
          <button
            @click="goHome"
            class="group relative inline-flex items-center justify-center px-4 py-3 rounded-2xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md transition-all gap-2 overflow-hidden whitespace-nowrap"
          >
            <div class="absolute inset-0 bg-white/10 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
            <HomeIcon class="w-4 h-4 relative z-10" />
            <span class="relative z-10">首页</span>
          </button>

          <button
            @click="goPlay"
            :disabled="!canEdit || isSaving"
            class="group relative inline-flex items-center justify-center px-4 py-3 rounded-2xl font-bold text-black bg-white hover:bg-neutral-200 transition-all shadow-[0_0_18px_rgba(255,255,255,0.25)] overflow-hidden disabled:opacity-40 disabled:cursor-not-allowed whitespace-nowrap"
          >
            <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/60 to-transparent -translate-x-full group-hover:translate-x-full transition-transform duration-700"></div>
            <Spline class="w-4 h-4 relative z-10" />
            <span class="ml-2 relative z-10">保存并游玩</span>
          </button>

          <button
            @click="void applyDraft()"
            :disabled="!canEdit || isSaving || (!dirty && playEntry !== 'import')"
            :class="(!canEdit || isSaving || (!dirty && playEntry !== 'import')) ? 'opacity-40 cursor-not-allowed' : ''"
            class="group relative inline-flex items-center justify-center px-4 py-3 rounded-2xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md transition-all gap-2 overflow-hidden disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <div class="absolute inset-0 bg-white/10 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
            <Save class="w-4 h-4 relative z-10" />
            <span class="relative z-10 whitespace-nowrap">保存</span>
          </button>

          <button
            v-if="canShare"
            @click="handleShare"
            :disabled="shareLoading"
            class="group relative inline-flex items-center justify-center px-4 py-3 rounded-2xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md transition-all gap-2 overflow-hidden disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <div class="absolute inset-0 bg-white/10 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
            <Share2 v-if="!isShared" class="w-4 h-4 relative z-10" />
            <Lock v-else class="w-4 h-4 relative z-10" />
            <span class="relative z-10 whitespace-nowrap">{{ shareLoading ? '处理中...' : (isShared ? '取消分享' : '分享') }}</span>
          </button>

          <button
            v-if="canEdit"
            @click="openImportModal"
            class="group relative inline-flex items-center justify-center px-4 py-3 rounded-2xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md transition-all gap-2 overflow-hidden"
          >
            <div class="absolute inset-0 bg-white/10 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
            <ImportIcon class="w-4 h-4 relative z-10" />
            <span class="relative z-10 whitespace-nowrap">导入</span>
          </button>

          <button
            @click="showJsonModal = true"
            class="group relative inline-flex items-center justify-center px-4 py-3 rounded-2xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md transition-all gap-2 overflow-hidden"
          >
            <div class="absolute inset-0 bg-white/10 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
            <FileJson class="w-4 h-4 relative z-10" />
            <span class="relative z-10 whitespace-nowrap">导出</span>
          </button>
        </div>
      </header>

      <Transition
        enter-active-class="animate-in fade-in slide-in-from-top-4 duration-300"
        leave-active-class="animate-out fade-out slide-out-to-top-2 duration-200"
      >
        <div
          v-if="toast"
          class="fixed top-6 left-1/2 -translate-x-1/2 z-[100] px-4 py-2.5 rounded-2xl border backdrop-blur-md shadow-2xl"
          :class="[
            toast.kind === 'success'
              ? 'border-green-500/30 bg-green-500/10 text-green-100'
              : toast.kind === 'error'
                ? 'border-red-500/30 bg-red-500/10 text-red-100'
                : 'border-white/15 bg-black/35 text-white/90',
          ]"
        >
          <div class="text-sm font-semibold">{{ toast.text }}</div>
        </div>
      </Transition>

      <div class="mt-8">
        <div v-if="isLoading" class="py-16 flex items-center justify-center">
          <CinematicLoader text="正在载入可编辑数据..." />
        </div>

        <div
          v-else-if="loadError"
          class="rounded-2xl border border-red-500/30 bg-red-500/10 p-5 backdrop-blur-md"
        >
          <div class="text-sm font-bold text-red-200">加载失败</div>
          <div class="mt-1 text-sm text-white/70">{{ loadError }}</div>
        </div>

        <div
          v-else-if="accessError"
          class="relative overflow-hidden rounded-3xl border border-white/10 bg-black/35 backdrop-blur-xl p-8 md:p-10 shadow-2xl"
        >
          <div class="absolute -inset-1 bg-gradient-to-r from-purple-600/25 via-fuchsia-600/25 to-cyan-600/25 blur-2xl opacity-60"></div>
          <div class="relative">
            <div class="text-xs tracking-[0.28em] uppercase text-white/55 font-semibold">Access</div>
            <div class="mt-4 text-2xl md:text-3xl font-black text-white/90">无法进入设计模式</div>
            <div class="mt-3 text-white/60 leading-relaxed max-w-2xl">{{ accessError }}</div>
            <div class="mt-6 flex flex-col sm:flex-row gap-3">
              <button
                @click="goHome"
                class="group relative inline-flex items-center justify-center px-8 py-3 rounded-2xl font-bold text-black bg-white hover:bg-neutral-200 transition-all shadow-[0_0_20px_rgba(255,255,255,0.28)] overflow-hidden"
              >
                <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/50 to-transparent -translate-x-full group-hover:translate-x-full transition-transform duration-700"></div>
                <span class="relative z-10">返回首页</span>
              </button>

              <button
                v-if="draft"
                @click="discardDraft"
                class="group relative inline-flex items-center justify-center px-8 py-3 rounded-2xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md transition-all gap-2 overflow-hidden"
              >
                <div class="absolute inset-0 bg-white/10 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
                <span class="relative z-10">重置草稿</span>
              </button>
            </div>
          </div>
        </div>

        <div v-else class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <section class="relative overflow-hidden rounded-3xl border border-white/10 bg-black/35 backdrop-blur-xl p-6 md:p-7 shadow-2xl">
            <div class="absolute -inset-1 bg-gradient-to-r from-cyan-600/20 via-purple-600/15 to-fuchsia-600/20 blur-2xl opacity-60"></div>
            <div class="relative">
              <div class="flex items-center justify-between gap-3">
                <div>
                  <div class="text-xs tracking-[0.28em] uppercase text-white/55 font-semibold">
                    Story
                  </div>
                  <h2 class="mt-2 text-xl md:text-2xl font-black text-white/90">故事剧情</h2>
                  <p class="mt-2 text-sm text-white/55 leading-relaxed">
                    这些字段与首页完全一致，并同步保存到本地。
                  </p>
                </div>
              </div>

              <div class="mt-6 space-y-5">
                <div class="space-y-2">
                  <div class="text-sm font-bold text-white/80">游戏主题</div>
                  <input
                    v-model="theme"
                    class="w-full px-4 py-3 rounded-2xl border border-white/10 bg-black/35 backdrop-blur-md text-white/90 placeholder:text-white/35 focus:outline-none focus:ring-2 focus:ring-purple-500/40"
                    placeholder="例如：赛博朋克背景下的硬汉侦探故事..."
                  />
                </div>

                <div class="space-y-2">
                  <div class="flex items-center justify-between gap-3">
                    <div class="text-sm font-bold text-white/80">剧情简介</div>
                    <div class="text-xs text-white/45">设计页禁止修改</div>
                  </div>
                  <textarea
                    v-model="synopsis"
                    rows="6"
                    readonly
                    class="w-full px-4 py-3 rounded-2xl border border-white/10 bg-black/35 backdrop-blur-md text-white/90 placeholder:text-white/35 focus:outline-none focus:ring-2 focus:ring-purple-500/40 opacity-70 cursor-not-allowed"
                    placeholder="描述故事的核心冲突、世界背景和开场氛围..."
                  ></textarea>
                </div>

                <div class="space-y-3">
                  <div class="flex items-center justify-between gap-3">
                    <div class="text-sm font-bold text-white/80">剧情类型 (多选)</div>
                    <div class="text-xs text-white/45">设计页禁止修改</div>
                  </div>
                  <div class="flex flex-wrap gap-2">
                    <button
                      v-for="g in availableGenres"
                      :key="g"
                      type="button"
                      disabled
                      :class="[
                        'px-3 py-2 rounded-xl border text-sm transition whitespace-nowrap opacity-70 cursor-not-allowed',
                        selectedGenres.includes(g)
                          ? 'bg-purple-600/70 border-purple-500/70 text-white'
                          : 'bg-black/25 border-white/10 text-white/70',
                      ]"
                    >
                      {{ g }}
                    </button>
                    <div class="flex items-center gap-2 opacity-70">
                      <input
                        v-model="customGenre"
                        disabled
                        placeholder="添加..."
                        class="px-3 py-2 rounded-xl text-sm bg-black/25 border border-white/10 text-white/90 focus:outline-none w-20"
                      />
                      <button
                        type="button"
                        disabled
                        class="px-2 py-2 rounded-xl border border-white/10 bg-black/25 text-white/70 text-sm cursor-not-allowed"
                      >
                        +
                      </button>
                    </div>
                  </div>
                </div>

                <div class="space-y-3">
                  <div class="flex items-center justify-between">
                    <div class="text-sm font-bold text-white/80">角色阵容</div>
                    <button
                      @click="characters = [...characters, { name: '新角色', description: '', gender: '其他', isMain: false }]"
                      class="group inline-flex items-center gap-2 px-3 py-2 rounded-xl border border-white/10 bg-black/35 hover:bg-black/55 transition"
                    >
                      <Plus class="w-4 h-4 text-white/80 group-hover:scale-110 transition-transform" />
                      <span class="text-sm text-white/80">添加</span>
                    </button>
                  </div>

                  <div class="space-y-3">
                    <div
                      v-for="(c, idx) in characters"
                      :key="idx"
                      class="rounded-2xl border border-white/10 bg-black/25 p-4 backdrop-blur-md"
                    >
                      <div class="flex flex-col md:flex-row gap-4">
                        <div class="flex items-center justify-center md:justify-start">
                          <CharacterAvatar
                            :name="c.name"
                            :gender="c.gender === '女' ? 'female' : c.gender === '男' ? 'male' : 'other'"
                            :avatarPath="c.avatarPath"
                            className="w-20 h-20"
                          />
                        </div>

                        <div class="flex-1">
                          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                            <input
                              v-model="c.name"
                              class="w-full px-3 py-2.5 rounded-xl border border-white/10 bg-black/35 text-white/90 focus:outline-none focus:ring-2 focus:ring-cyan-500/30"
                              placeholder="名字"
                            />
                            <select
                              v-model="c.gender"
                              class="w-full px-3 py-2.5 rounded-xl border border-white/10 bg-black/35 text-white/90 focus:outline-none focus:ring-2 focus:ring-cyan-500/30"
                            >
                              <option value="男">男</option>
                              <option value="女">女</option>
                              <option value="其他">其他</option>
                            </select>
                          </div>

                          <textarea
                            v-model="c.description"
                            rows="3"
                            class="mt-3 w-full px-3 py-2.5 rounded-xl border border-white/10 bg-black/35 text-white/90 focus:outline-none focus:ring-2 focus:ring-cyan-500/30"
                            placeholder="身份与性格描述"
                          ></textarea>

                          <div class="mt-3 flex flex-col md:flex-row md:items-center md:justify-between gap-3">
                            <div class="flex items-center justify-between gap-3">
                              <label class="inline-flex items-center gap-2 text-sm text-white/70">
                                <input
                                  type="checkbox"
                                  v-model="c.isMain"
                                  class="accent-purple-500"
                                />
                                主角
                              </label>

                              <div class="flex items-center gap-2">
                                <input
                                  :id="`avatar-upload-${idx}`"
                                  type="file"
                                  accept="image/*"
                                  class="hidden"
                                  @change="setCharacterAvatar(idx, $event)"
                                />
                                <label
                                  :for="`avatar-upload-${idx}`"
                                  class="inline-flex items-center gap-2 px-3 py-2 rounded-xl border border-white/10 bg-black/35 hover:bg-black/55 transition text-sm text-white/80 cursor-pointer whitespace-nowrap"
                                >
                                  上传头像
                                </label>
                                <button
                                  type="button"
                                  @click="clearCharacterAvatar(idx)"
                                  :disabled="!c.avatarPath"
                                  class="inline-flex items-center gap-2 px-3 py-2 rounded-xl border border-white/10 bg-black/25 hover:bg-black/45 transition text-sm text-white/70 disabled:opacity-40 disabled:cursor-not-allowed whitespace-nowrap"
                                >
                                  清除
                                </button>
                              </div>
                            </div>

                            <div class="flex flex-col items-end gap-1">
                              <button
                                type="button"
                                @click="removeCharacter(idx)"
                                :disabled="!canRemoveCharacter(idx)"
                                :class="[
                                  'group inline-flex items-center gap-2 px-3 py-2 rounded-xl border transition whitespace-nowrap',
                                  canRemoveCharacter(idx)
                                    ? 'border-red-500/20 bg-red-500/10 hover:bg-red-500/15'
                                    : 'border-white/10 bg-white/5 opacity-50 cursor-not-allowed',
                                ]"
                              >
                                <Trash2 class="w-4 h-4 text-red-200 group-hover:scale-110 transition-transform" />
                                <span class="text-sm text-red-100">移除</span>
                              </button>
                              <div v-if="!canRemoveCharacter(idx)" class="text-xs text-white/45">
                                该角色已在节点中引用，需先移除引用
                              </div>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>

              </div>
            </div>
          </section>

          <section class="relative overflow-hidden rounded-3xl border border-white/10 bg-black/35 backdrop-blur-xl p-6 md:p-7 shadow-2xl">
            <div class="absolute -inset-1 bg-gradient-to-r from-purple-600/20 via-fuchsia-600/20 to-cyan-600/20 blur-2xl opacity-60"></div>
            <div class="relative">
              <div class="flex items-start justify-between gap-3">
                <div>
                  <div class="text-xs tracking-[0.28em] uppercase text-white/55 font-semibold">
                    Story
                  </div>
                  <h2 class="mt-2 text-xl md:text-2xl font-black text-white/90">节点与结局</h2>
                  <p class="mt-2 text-sm text-white/55 leading-relaxed">
                    点击节点打开弹窗编辑，结构以树形方式呈现；可新增/删除/改名。
                  </p>
                </div>

                <button
                  @click="addNodeAtRoot"
                  :disabled="!canEdit"
                  class="group inline-flex items-center gap-2 px-4 py-2.5 rounded-2xl border border-white/10 bg-black/35 hover:bg-black/55 transition-all disabled:opacity-40 disabled:cursor-not-allowed"
                >
                  <Plus class="w-4 h-4 text-white/80 group-hover:scale-110 transition-transform" />
                  <span class="text-sm font-bold text-white/85 whitespace-nowrap">新增节点</span>
                </button>
              </div>

              <div class="mt-6 space-y-6">
                <div class="flex flex-wrap gap-2">
                  <span class="px-3 py-1 rounded-full border border-white/10 bg-white/5 text-xs text-white/70">nodes: {{ stats.nodes }}</span>
                  <span class="px-3 py-1 rounded-full border border-white/10 bg-white/5 text-xs text-white/70">endings: {{ stats.endings }}</span>
                  <span class="px-3 py-1 rounded-full border border-white/10 bg-white/5 text-xs text-white/70">characters: {{ stats.characters }}</span>
                </div>

                <section class="rounded-2xl border border-white/10 bg-black/25 p-4 overflow-hidden">
                  <div class="flex items-center justify-between gap-4">
                    <div>
                      <div class="text-sm font-bold text-white/80">剧情树</div>
                      <div class="text-xs text-white/45 font-mono">start: {{ startNodeId || '-' }}</div>
                    </div>
                    <div class="flex items-center gap-2">
                      <button class="px-3 py-1 rounded-lg border border-white/10 bg-white/5 text-xs text-white/70 hover:bg-white/10 transition" @click="fitTree">适配视图</button>
                      <button class="px-3 py-1 rounded-lg border border-white/10 bg-white/5 text-xs text-white/70 hover:bg-white/10 transition" @click="resetView">重置</button>
                    </div>
                  </div>

                  <div
                    ref="treeWrapEl"
                    class="mt-4 rounded-xl border border-white/10 bg-black/55 overflow-hidden h-[640px] md:h-[720px] relative touch-none"
                    @wheel="onWheel"
                    @pointerdown="onPointerDown"
                    @pointermove="onPointerMove"
                    @pointerup="onPointerUp"
                    @pointercancel="onPointerUp"
                  >
                    <svg class="absolute inset-0 w-full h-full select-none" :style="{ cursor: dragging ? 'grabbing' : 'grab' }">
                      <defs>
                        <linearGradient id="edge-gradient" gradientUnits="userSpaceOnUse" x1="0%" y1="0%" x2="100%" y2="0%">
                          <stop offset="0%" stop-color="#9333ea" stop-opacity="0.3" />
                          <stop offset="100%" stop-color="#22d3ee" stop-opacity="0.3" />
                        </linearGradient>
                        <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto">
                          <polygon points="0 0, 10 3.5, 0 7" fill="#64748b" opacity="0.5" />
                        </marker>
                      </defs>

                      <g :transform="`translate(${pan.x} ${pan.y}) scale(${zoom})`">
                        <g>
                          <path
                            v-for="(e, idx) in treeGraph.edges"
                            :key="idx"
                            :d="(() => {
                              const a = treeGraph.nodes.find(n => n.id === e.from);
                              const b = treeGraph.nodes.find(n => n.id === e.to);
                              if (!a || !b) return '';
                              const ax = a.x + a.w;
                              const ay = a.y + a.h / 2;
                              const bx = b.x;
                              const by = b.y + b.h / 2;
                              const mx = (ax + bx) / 2;
                              return `M ${ax} ${ay} C ${mx} ${ay}, ${mx} ${by}, ${bx} ${by}`;
                            })()"
                            :stroke="(highlighted.has(e.from) && highlighted.has(e.to)) ? '#d946ef' : 'url(#edge-gradient)'"
                            :stroke-width="(highlighted.has(e.from) && highlighted.has(e.to)) ? 3 : 1.5"
                            :stroke-opacity="(highlighted.has(e.from) && highlighted.has(e.to)) ? 0.8 : 0.3"
                            fill="none"
                            class="transition-all duration-500"
                            marker-end="url(#arrowhead)"
                          />
                        </g>

                        <g>
                          <foreignObject
                            v-for="n in treeGraph.nodes"
                            :key="n.id"
                            :x="n.x"
                            :y="n.y"
                            :width="n.w"
                            :height="n.h"
                            class="overflow-visible"
                          >
                            <div
                              data-node
                              @click.stop="onNodeClick(n.id)"
                              @pointerenter="hoveredId = n.id"
                              @pointerleave="hoveredId = ''"
                              :class="[
                                'w-full h-full rounded-xl border backdrop-blur-md flex flex-col justify-center px-4 py-2 transition-all duration-300 cursor-pointer shadow-lg group hover:scale-105',
                                n.kind === 'ending'
                                  ? 'bg-cyan-900/20 border-cyan-500/30 hover:border-cyan-400'
                                  : 'bg-neutral-900/40 border-purple-500/20 hover:border-purple-400',
                                highlighted.has(n.id) ? 'ring-2 ring-purple-500/50 shadow-[0_0_15px_rgba(168,85,247,0.3)]' : ''
                              ]"
                            >
                              <div class="flex items-center justify-between mb-1">
                                <span :class="['text-[10px] font-mono uppercase tracking-wider', n.kind === 'ending' ? 'text-cyan-400' : 'text-purple-400']">
                                  {{ n.kind === 'ending' ? 'ENDING' : 'NODE' }}
                                </span>
                                <div v-if="highlighted.has(n.id)" class="w-1.5 h-1.5 rounded-full bg-green-400 animate-pulse shadow-[0_0_5px_rgba(74,222,128,0.8)]"></div>
                              </div>
                              <div class="text-xs font-bold text-white/90 truncate font-mono">{{ n.label }}</div>
                              <div class="absolute inset-0 rounded-xl bg-gradient-to-r from-white/0 via-white/5 to-white/0 opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none"></div>
                            </div>
                          </foreignObject>
                        </g>
                      </g>
                    </svg>

                    <div
                      v-if="selectedNodeInfo"
                      class="absolute right-4 top-4 w-[320px] max-w-[90%] rounded-2xl border border-white/10 bg-black/90 backdrop-blur-xl p-5 shadow-2xl ring-1 ring-white/5 z-20"
                      @pointerdown.stop
                      @mousedown.stop
                      @click.stop
                    >
                      <div class="flex items-center justify-between gap-2 mb-4">
                        <div class="text-xs tracking-[0.22em] uppercase text-white/50 font-semibold">{{ selectedNodeInfo.kind === 'ending' ? 'Ending' : 'Node' }}</div>
                        <button class="text-white/40 hover:text-white p-1 rounded-full hover:bg-white/10 transition-colors" @click.stop="selectedId = ''">
                          <X class="w-4 h-4" />
                        </button>
                      </div>

                      <div class="font-mono text-white/90 break-all text-sm font-bold border-b border-white/10 pb-3 mb-3">{{ selectedNodeInfo.id }}</div>

                      <template v-if="selectedNodeInfo.kind === 'ending'">
                        <div class="mt-3 text-sm text-white/80 leading-relaxed">{{ selectedNodeInfo.description }}</div>
                        <div class="mt-3 text-xs text-white/55">type: {{ selectedNodeInfo.type }}</div>
                        <button
                          class="mt-4 w-full px-4 py-2 rounded-xl border border-white/10 bg-white/10 hover:bg-white/20 text-white text-sm font-semibold transition disabled:opacity-40 disabled:cursor-not-allowed"
                          :disabled="!canEdit"
                          @click.stop="openEnding(selectedNodeInfo.id)"
                        >
                          编辑结局
                        </button>
                      </template>
                      <template v-else>
                        <div v-if="selectedNodeInfo.characters?.length" class="mt-3 text-xs text-white/55">characters: {{ selectedNodeInfo.characters.join(' / ') }}</div>
                        <div class="mt-3 text-sm text-white/80 leading-relaxed max-h-[140px] overflow-auto custom-scrollbar">{{ selectedNodeInfo.content }}</div>
                        <div v-if="selectedNodeInfo.choices?.length" class="mt-3">
                          <div class="text-xs text-white/50 tracking-[0.18em] uppercase font-semibold">choices</div>
                          <div class="mt-2 space-y-1">
                            <button
                              v-for="(c, idx) in selectedNodeInfo.choices"
                              :key="idx"
                              class="w-full text-left px-3 py-2 rounded-lg border border-white/10 bg-white/5 hover:bg-white/10 transition"
                              @click.stop="selectedId = c.to"
                            >
                              <div class="text-xs text-white/80">{{ c.text }}</div>
                              <div class="text-[11px] font-mono text-white/55">→ {{ c.to }}</div>
                            </button>
                          </div>
                        </div>
                        <button
                          class="mt-4 w-full px-4 py-2 rounded-xl border border-white/10 bg-white/10 hover:bg-white/20 text-white text-sm font-semibold transition disabled:opacity-40 disabled:cursor-not-allowed"
                          :disabled="!canEdit"
                          @click.stop="openNode(selectedNodeInfo.id)"
                        >
                          编辑节点
                        </button>
                      </template>
                    </div>
                  </div>

                  <div v-if="orphanNodeIds.length" class="mt-4 rounded-2xl border border-white/10 bg-black/25 p-4">
                    <div class="text-sm font-bold text-white/80">孤立节点</div>
                    <div class="mt-2 text-xs text-white/55 leading-relaxed">
                      孤立节点指：从 start 出发不可达，或没有任何节点选项指向它。它们不会出现在正常剧情流程中，会增加维护成本，建议删除或重新连接到某个分支。
                    </div>
                    <div class="mt-3 flex flex-wrap gap-2">
                      <div
                        v-for="id in orphanNodeIds"
                        :key="id"
                        class="inline-flex items-center gap-1.5 rounded-full border border-white/10 bg-black/35 px-2 py-1.5"
                      >
                        <button
                          type="button"
                          @click="openNode(id)"
                          :disabled="!canEdit"
                          class="px-2 py-0.5 text-xs text-white/75 hover:text-white transition disabled:opacity-40 disabled:cursor-not-allowed"
                        >
                          {{ id }}
                        </button>
                        <span class="text-[10px] text-white/45">{{ orphanNodeReasonMap.get(id) }}</span>
                        <button
                          type="button"
                          @click="deleteNode(id)"
                          :disabled="!canEdit"
                          class="p-1 rounded-full hover:bg-white/10 transition disabled:opacity-40 disabled:cursor-not-allowed"
                          title="删除"
                        >
                          <Trash2 class="w-3.5 h-3.5 text-red-200" />
                        </button>
                      </div>
                    </div>
                  </div>
                </section>

                <section class="mt-6 rounded-2xl border border-white/10 bg-black/25 p-4">
                  <div class="flex items-center justify-between">
                    <div class="text-sm font-bold text-white/80">结局</div>
                    <button
                      @click="addEnding"
                      :disabled="!canEdit"
                      class="group inline-flex items-center gap-2 px-3 py-2 rounded-xl border border-white/10 bg-black/35 hover:bg-black/55 transition disabled:opacity-40 disabled:cursor-not-allowed"
                    >
                      <Plus class="w-4 h-4 text-white/80 group-hover:scale-110 transition-transform" />
                      <span class="text-sm text-white/80">新增结局</span>
                    </button>
                  </div>

                  <div class="mt-3 rounded-2xl border border-white/10 bg-black/25 p-4 max-h-[420px] overflow-auto no-scrollbar">
                    <div v-if="endingKeys.length === 0" class="text-sm text-white/60">暂无结局</div>
                    <div v-else class="space-y-2">
                      <button
                        v-for="key in endingKeys"
                        :key="key"
                        @click="openEnding(key)"
                        :disabled="!canEdit"
                        class="w-full text-left px-4 py-3 rounded-2xl border border-white/10 bg-black/35 hover:bg-black/55 transition disabled:opacity-40 disabled:cursor-not-allowed"
                      >
                        <div class="flex items-center justify-between gap-3">
                          <div class="text-sm font-black text-white/90 truncate">{{ key }}</div>
                          <div class="text-xs font-mono text-white/50">{{ draft?.endings?.[key]?.type }}</div>
                        </div>
                        <div class="mt-1 text-xs text-white/55 line-clamp-2">
                          {{ draft?.endings?.[key]?.description }}
                        </div>
                      </button>
                    </div>
                  </div>
                </section>

                <section class="mt-6 rounded-2xl border border-white/10 bg-black/25 p-4">
                  <div class="text-sm font-bold text-white/80">状态</div>
                  <div class="mt-2 flex flex-wrap gap-2 text-xs">
                    <div class="px-3 py-1 rounded-full border border-white/10 bg-white/5 text-white/70">
                      entry: <span class="font-mono">{{ playEntry }}</span>
                    </div>
                    <div class="px-3 py-1 rounded-full border border-white/10 bg-white/5 text-white/70">
                      owner: <span class="font-mono">{{ isOwner ? 'YES' : 'NO' }}</span>
                    </div>
                    <div
                      class="px-3 py-1 rounded-full border border-white/10 bg-white/5"
                      :class="dirty ? 'text-yellow-200' : 'text-green-200'"
                    >
                      <span class="font-mono">{{ dirty ? 'DIRTY' : 'CLEAN' }}</span>
                    </div>
                  </div>
                </section>

                <div class="flex flex-col sm:flex-row gap-3">
                  <button
                    @click="void applyDraft()"
                    :disabled="!canEdit || !dirty"
                    :class="(!canEdit || !dirty) ? 'opacity-40 cursor-not-allowed' : ''"
                    class="group relative inline-flex items-center justify-center px-6 py-3 rounded-2xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md transition-all gap-2 overflow-hidden disabled:opacity-40 disabled:cursor-not-allowed"
                  >
                    <div class="absolute inset-0 bg-white/10 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
                    <Save class="w-4 h-4 relative z-10" />
                    <span class="relative z-10">保存到本地</span>
                  </button>

                  <button
                    @click="discardDraft"
                    :disabled="!dirty"
                    class="group relative inline-flex items-center justify-center px-6 py-3 rounded-2xl font-bold text-white/90 border border-white/10 bg-black/25 hover:bg-black/45 backdrop-blur-md transition-all gap-2 overflow-hidden disabled:opacity-40 disabled:cursor-not-allowed"
                  >
                    <div class="absolute inset-0 bg-white/10 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
                    <span class="relative z-10">放弃更改</span>
                  </button>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>

    <Transition enter-active-class="animate-in fade-in duration-200" leave-active-class="animate-out fade-out duration-150">
      <div v-if="nodeModalOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4">
        <div class="absolute inset-0 bg-black/80 backdrop-blur-md" @click="closeNodeModal"></div>
        <div class="w-full max-w-3xl bg-neutral-900/90 border border-white/10 rounded-2xl overflow-hidden shadow-2xl relative z-10">
          <div class="px-5 py-4 flex items-center justify-between border-b border-white/10">
            <div class="flex items-center gap-2">
              <Pencil class="w-4 h-4 text-cyan-400" />
              <div class="font-bold text-white">编辑节点</div>
            </div>
            <button @click="closeNodeModal" class="p-2 rounded-lg hover:bg-white/5 transition-colors">
              <X class="w-5 h-5 text-white/70" />
            </button>
          </div>

          <div class="p-5 md:p-6 space-y-5 max-h-[75vh] overflow-auto no-scrollbar">
            <div v-if="!editingNode" class="text-sm text-white/60">节点不存在</div>

            <div v-else class="space-y-5">
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div class="space-y-2">
                  <div class="text-sm font-bold text-white/80">节点 ID</div>
                  <input
                    :value="editingNodeId || ''"
                    @change="renameNodeIfNeeded(String(editingNodeId || ''), String(($event.target as HTMLInputElement).value || ''))"
                    class="w-full px-4 py-3 rounded-2xl border border-white/10 bg-black/35 text-white/90 focus:outline-none focus:ring-2 focus:ring-purple-500/40"
                    :disabled="!canEdit"
                  />
                </div>

                <div class="space-y-2">
                  <div class="text-sm font-bold text-white/80">角色（选择）</div>
                  <div v-if="nodeCharacterOptions.length === 0" class="text-sm text-white/50">
                    请先在「故事剧情」里添加角色
                  </div>
                  <div v-else class="flex flex-wrap gap-2">
                    <button
                      v-for="name in nodeCharacterOptions"
                      :key="name"
                      type="button"
                      @click="toggleEditingNodeCharacter(name)"
                      :disabled="!canEdit"
                      :class="[
                        'px-3 py-2 rounded-xl border text-sm transition whitespace-nowrap',
                        editingNodeCharacterNameSet.has(name)
                          ? 'bg-purple-600/70 border-purple-500/70 text-white'
                          : 'bg-black/25 border-white/10 text-white/70 hover:bg-black/45',
                        !canEdit ? 'opacity-40 cursor-not-allowed' : '',
                      ]"
                    >
                      {{ name }}
                    </button>
                  </div>
                </div>
              </div>

              <div class="space-y-2">
                <div class="text-sm font-bold text-white/80">内容</div>
                <textarea
                  :value="editingNode.content"
                  @input="updateEditingNodeContent(String(editingNodeId || ''), $event)"
                  rows="6"
                  class="w-full px-4 py-3 rounded-2xl border border-white/10 bg-black/35 text-white/90 focus:outline-none focus:ring-2 focus:ring-purple-500/40"
                  :disabled="!canEdit"
                ></textarea>
              </div>

              <div class="rounded-2xl border border-white/10 bg-black/25 p-4">
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-white/80">选项</div>
                  <button
                    type="button"
                    @click="addChoice(String(editingNodeId || ''))"
                    :disabled="!canEdit"
                    class="group inline-flex items-center gap-2 px-3 py-2 rounded-xl border border-white/10 bg-black/35 hover:bg-black/55 transition disabled:opacity-40 disabled:cursor-not-allowed"
                  >
                    <Plus class="w-4 h-4 text-white/80 group-hover:scale-110 transition-transform" />
                    <span class="text-sm text-white/80">新增选项</span>
                  </button>
                </div>

                <div class="mt-3 space-y-3">
                  <div
                    v-for="(c, idx) in editingNode.choices"
                    :key="idx"
                    class="rounded-2xl border border-white/10 bg-black/35 p-4"
                  >
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                      <input
                        :value="c.text"
                        @input="updateChoice(String(editingNodeId || ''), idx, { text: String(($event.target as HTMLInputElement).value || '') })"
                        class="w-full px-3 py-2.5 rounded-xl border border-white/10 bg-black/35 text-white/90 focus:outline-none focus:ring-2 focus:ring-cyan-500/30"
                        placeholder="选项文本"
                        :disabled="!canEdit"
                      />

                      <select
                        :value="c.nextNodeId"
                        @change="updateChoice(String(editingNodeId || ''), idx, { nextNodeId: String(($event.target as HTMLSelectElement).value || '') })"
                        class="w-full px-3 py-2.5 rounded-xl border border-white/10 bg-black/35 text-white/90 focus:outline-none focus:ring-2 focus:ring-cyan-500/30"
                        :disabled="!canEdit"
                      >
                        <option value="">（未设置）</option>
                        <option v-for="id in availableNextIds" :key="id" :value="id">{{ getNextIdLabel(id) }}</option>
                      </select>
                    </div>

                    <div class="mt-3 grid grid-cols-1 md:grid-cols-2 gap-3">
                      <select
                        :value="String(c.affinityEffect?.characterId || '')"
                        @change="(e) => {
                          const v = String((e.target as HTMLSelectElement).value || '').trim();
                          if (!v) {
                            updateChoice(String(editingNodeId || ''), idx, { affinityEffect: undefined });
                            return;
                          }
                          const d = clamp(Number(c.affinityEffect?.delta ?? 0), -20, 20);
                          updateChoice(String(editingNodeId || ''), idx, { affinityEffect: { characterId: v, delta: d } });
                        }"
                        class="w-full px-3 py-2.5 rounded-xl border border-white/10 bg-black/35 text-white/90 focus:outline-none focus:ring-2 focus:ring-purple-500/30"
                        :disabled="!canEdit || affinityTargetOptions.length === 0"
                      >
                        <option value="">（不影响好感度）</option>
                        <option v-for="name in affinityTargetOptions" :key="name" :value="name">{{ name }}</option>
                      </select>

                      <input
                        type="number"
                        min="-20"
                        max="20"
                        step="1"
                        :value="Number(c.affinityEffect?.delta ?? 0)"
                        @input="(e) => {
                          const raw = String((e.target as HTMLInputElement).value || '').trim();
                          const num = raw === '' ? 0 : Number(raw);
                          const next = clamp(Number.isFinite(num) ? Math.round(num) : 0, -20, 20);
                          const who = String(c.affinityEffect?.characterId || '').trim();
                          if (!who) return;
                          updateChoice(String(editingNodeId || ''), idx, { affinityEffect: { characterId: who, delta: next } });
                        }"
                        class="w-full px-3 py-2.5 rounded-xl border border-white/10 bg-black/35 text-white/90 focus:outline-none focus:ring-2 focus:ring-purple-500/30"
                        :disabled="!canEdit || !String(c.affinityEffect?.characterId || '').trim()"
                        placeholder="好感度变化（-20 ~ 20）"
                      />
                    </div>

                    <div class="mt-3 flex items-center justify-between">
                      <button
                        type="button"
                        @click="openNode(String(c.nextNodeId || ''))"
                        :disabled="!canEdit || !String(c.nextNodeId || '').trim() || String(c.nextNodeId || '').trim() === 'END' || Boolean(draft?.endings?.[String(c.nextNodeId || '').trim()])"
                        class="inline-flex items-center gap-2 px-3 py-2 rounded-xl border border-white/10 bg-black/25 hover:bg-black/45 text-sm text-white/70 transition disabled:opacity-40 disabled:cursor-not-allowed"
                      >
                        <span>打开目标节点</span>
                      </button>

                      <button
                        type="button"
                        @click="removeChoice(String(editingNodeId || ''), idx)"
                        :disabled="!canEdit"
                        class="group inline-flex items-center gap-2 px-3 py-2 rounded-xl border border-red-500/20 bg-red-500/10 hover:bg-red-500/15 transition disabled:opacity-40 disabled:cursor-not-allowed"
                      >
                        <Trash2 class="w-4 h-4 text-red-200 group-hover:scale-110 transition-transform" />
                        <span class="text-sm text-red-100">删除选项</span>
                      </button>
                    </div>
                  </div>
                </div>


              </div>
            </div>
          </div>

          <div class="px-5 py-4 bg-black/20 border-t border-white/10 flex items-center justify-between">
            <div class="text-xs text-white/50 font-mono">{{ editingNodeId || '' }}</div>
            <button @click="closeNodeModal" class="px-6 py-2 rounded-xl bg-white/10 hover:bg-white/20 text-white font-medium transition-colors">
              完成
            </button>
          </div>
        </div>
      </div>
    </Transition>

    <Transition enter-active-class="animate-in fade-in duration-200" leave-active-class="animate-out fade-out duration-150">
      <div v-if="endingModalOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4">
        <div class="absolute inset-0 bg-black/80 backdrop-blur-md" @click="closeEndingModal"></div>
        <div class="w-full max-w-2xl bg-neutral-900/90 border border-white/10 rounded-2xl overflow-hidden shadow-2xl relative z-10">
          <div class="px-5 py-4 flex items-center justify-between border-b border-white/10">
            <div class="flex items-center gap-2">
              <Pencil class="w-4 h-4 text-fuchsia-400" />
              <div class="font-bold text-white">编辑结局</div>
            </div>
            <button @click="closeEndingModal" class="p-2 rounded-lg hover:bg-white/5 transition-colors">
              <X class="w-5 h-5 text-white/70" />
            </button>
          </div>

          <div class="p-5 md:p-6 space-y-5">
            <div v-if="!editingEnding" class="text-sm text-white/60">结局不存在</div>

            <div v-else class="space-y-5">
              <div class="space-y-2">
                <div class="text-sm font-bold text-white/80">Key</div>
                <input
                  :value="editingEndingKey || ''"
                  @change="renameEndingIfNeeded(String(editingEndingKey || ''), String(($event.target as HTMLInputElement).value || ''))"
                  class="w-full px-4 py-3 rounded-2xl border border-white/10 bg-black/35 text-white/90 focus:outline-none focus:ring-2 focus:ring-fuchsia-500/30"
                  :disabled="!canEdit"
                />
              </div>

              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div class="space-y-2">
                  <div class="text-sm font-bold text-white/80">类型</div>
                  <select
                    :value="editingEnding.type"
                    @change="patchDraft((d) => { const k = String(editingEndingKey || ''); if (!k) return; d.endings = d.endings || {}; d.endings[k] = { ...d.endings[k]!, type: ($event.target as HTMLSelectElement).value as Ending['type'] }; })"
                    class="w-full px-4 py-3 rounded-2xl border border-white/10 bg-black/35 text-white/90 focus:outline-none focus:ring-2 focus:ring-fuchsia-500/30"
                    :disabled="!canEdit"
                  >
                    <option value="good">good</option>
                    <option value="neutral">neutral</option>
                    <option value="bad">bad</option>
                  </select>
                </div>

                <div class="space-y-2">
                  <div class="text-sm font-bold text-white/80">绑定节点（可选）</div>
                  <input
                    :value="editingEnding.nodeId || ''"
                    @input="patchDraft((d) => { const k = String(editingEndingKey || ''); if (!k) return; d.endings = d.endings || {}; d.endings[k] = { ...d.endings[k]!, nodeId: String(($event.target as HTMLInputElement).value || '') || undefined }; })"
                    class="w-full px-4 py-3 rounded-2xl border border-white/10 bg-black/35 text-white/90 focus:outline-none focus:ring-2 focus:ring-fuchsia-500/30"
                    :disabled="!canEdit"
                    placeholder="例如：start 或 12"
                  />
                </div>
              </div>

              <div class="space-y-2">
                <div class="text-sm font-bold text-white/80">描述</div>
                <textarea
                  :value="editingEnding.description"
                  @input="patchDraft((d) => { const k = String(editingEndingKey || ''); if (!k) return; d.endings = d.endings || {}; d.endings[k] = { ...d.endings[k]!, description: String(($event.target as HTMLTextAreaElement).value || '') }; })"
                  rows="5"
                  class="w-full px-4 py-3 rounded-2xl border border-white/10 bg-black/35 text-white/90 focus:outline-none focus:ring-2 focus:ring-fuchsia-500/30"
                  :disabled="!canEdit"
                ></textarea>
              </div>

              <div class="flex flex-col sm:flex-row gap-3">
                <button
                  @click="deleteEnding(String(editingEndingKey || ''))"
                  :disabled="!canEdit"
                  class="group inline-flex items-center justify-center gap-2 px-4 py-3 rounded-2xl font-bold text-white/85 border border-red-500/20 bg-red-500/10 hover:bg-red-500/15 transition disabled:opacity-40 disabled:cursor-not-allowed"
                >
                  <Trash2 class="w-4 h-4" />
                  <span>删除结局</span>
                </button>
              </div>
            </div>
          </div>

          <div class="px-5 py-4 bg-black/20 border-t border-white/10 flex items-center justify-between">
            <div class="text-xs text-white/50 font-mono">{{ editingEndingKey || '' }}</div>
            <button @click="closeEndingModal" class="px-6 py-2 rounded-xl bg-white/10 hover:bg-white/20 text-white font-medium transition-colors">
              完成
            </button>
          </div>
        </div>
      </div>
    </Transition>

    <Teleport to="body">
      <div
        v-if="showImportModal"
        class="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm"
        @click.self="closeImportModal"
      >
        <div class="relative w-full max-w-3xl rounded-2xl border border-white/10 bg-[#0a0a0a] shadow-2xl overflow-hidden flex flex-col max-h-[85vh]">
          <div class="flex items-center justify-between px-6 py-4 border-b border-white/10 bg-white/5">
            <div class="flex items-center gap-2">
              <ImportIcon class="w-5 h-5 text-cyan-300" />
              <span class="font-bold text-white/90">导入并覆盖剧情</span>
            </div>
            <button @click="closeImportModal" class="text-white/50 hover:text-white transition">
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="flex-1 overflow-auto p-6 space-y-4 custom-scrollbar">
            <div class="flex items-center gap-2">
              <button
                @click="importTab = 'paste'"
                :class="[
                  'px-4 py-2 rounded-full text-sm font-semibold border transition-all',
                  importTab === 'paste'
                    ? 'bg-purple-600/30 border-purple-500/40 text-white'
                    : 'bg-black/30 border-white/10 text-white/60 hover:text-white hover:border-purple-500/30',
                ]"
              >
                手动输入
              </button>
              <button
                @click="importTab = 'file'"
                :class="[
                  'px-4 py-2 rounded-full text-sm font-semibold border transition-all',
                  importTab === 'file'
                    ? 'bg-purple-600/30 border-purple-500/40 text-white'
                    : 'bg-black/30 border-white/10 text-white/60 hover:text-white hover:border-purple-500/30',
                ]"
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

            <div v-if="importError" class="bg-red-500/10 border border-red-500/20 text-red-500 p-3 rounded-xl text-sm text-center">{{ importError }}</div>

            <div class="rounded-xl border border-white/10 bg-black/35 px-4 py-3 text-xs text-white/55 leading-relaxed">
              导入会直接覆盖当前设计器中的剧情数据；创建者模式下可选择“覆盖并保存”将内容写回数据库。
            </div>
          </div>

          <div class="p-6 border-t border-white/10 bg-white/5 flex flex-col sm:flex-row justify-end gap-3">
            <button
              @click="confirmImportOverwriteSave"
              :disabled="isImportApplying || securityLocked || !canEdit || playEntry !== 'owner'"
              class="flex items-center justify-center gap-2 px-5 py-2.5 rounded-xl bg-purple-600/30 border border-purple-500/30 text-white/90 font-bold hover:bg-purple-600/40 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <span>{{ isImportApplying ? '处理中...' : '覆盖并保存' }}</span>
            </button>

            <button
              @click="confirmImportOverwriteLocal"
              :disabled="isImportApplying || !canEdit"
              class="flex items-center justify-center gap-2 px-5 py-2.5 rounded-xl bg-black/35 border border-white/10 text-white/90 font-bold hover:bg-black/55 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <span>{{ isImportApplying ? '处理中...' : '仅覆盖到本地' }}</span>
            </button>

            <button
              @click="closeImportModal"
              class="px-5 py-2.5 rounded-xl bg-white/10 hover:bg-white/20 text-white/80 font-medium transition-colors"
            >
              关闭
            </button>
          </div>
        </div>
      </div>

      <div
        v-if="showJsonModal"
        class="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm"
        @click.self="showJsonModal = false"
      >
        <div class="relative w-full max-w-2xl rounded-2xl border border-white/10 bg-[#0a0a0a] shadow-2xl overflow-hidden flex flex-col max-h-[80vh]">
          <div class="flex items-center justify-between px-6 py-4 border-b border-white/10 bg-white/5">
            <div class="flex items-center gap-2">
              <FileJson class="w-5 h-5 text-purple-400" />
              <span class="font-bold text-white/90">完整剧情信息导出</span>
            </div>
            <button @click="showJsonModal = false" class="text-white/50 hover:text-white transition">
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="flex-1 overflow-auto p-6 custom-scrollbar">
            <pre class="font-mono text-xs md:text-sm text-emerald-400 bg-black/50 p-4 rounded-xl border border-white/5 whitespace-pre-wrap break-all">{{ jsonContent }}</pre>
          </div>

          <div class="p-6 border-t border-white/10 bg-white/5 flex justify-end gap-3">
            <button @click="showJsonModal = false" class="px-4 py-2 rounded-lg text-sm font-medium text-white/60 hover:text-white transition">关闭</button>
            <button @click="copyJson" class="flex items-center gap-2 px-4 py-2 rounded-lg bg-white/10 hover:bg-white/20 text-white text-sm font-medium transition border border-white/10">
              <Copy class="w-4 h-4" />
              复制 JSON
            </button>
            <button @click="downloadJson" class="flex items-center gap-2 px-4 py-2 rounded-lg bg-white/10 hover:bg-white/20 text-white text-sm font-medium transition border border-white/10">
              <Download class="w-4 h-4" />
              下载 JSON
            </button>
          </div>
        </div>
      </div>

      <div
        v-if="showShareModal"
        class="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm"
        @click.self="showShareModal = false"
      >
        <div class="relative w-full max-w-lg rounded-2xl border border-white/10 bg-[#0a0a0a] shadow-2xl overflow-hidden flex flex-col">
          <div class="flex items-center justify-between px-6 py-4 border-b border-white/10 bg-white/5">
            <div class="flex items-center gap-2">
              <Share2 class="w-5 h-5 text-cyan-300" />
              <span class="font-bold text-white/90">剧情已分享</span>
            </div>
            <button @click="showShareModal = false" class="text-white/50 hover:text-white transition">
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="p-6 space-y-4">
            <div class="text-sm text-white/70 leading-relaxed">
              任何拥有链接的人都可以体验此剧情；再次点击“取消分享”可随时撤回。
            </div>

            <div class="rounded-xl border border-white/10 bg-black/35 px-4 py-3">
              <div class="text-white/50 text-xs tracking-wider uppercase">share link</div>
              <div class="mt-2 font-mono text-white/90 break-all text-sm">{{ shareLink }}</div>
            </div>

            <div v-if="sharedRecordId" class="rounded-xl border border-white/10 bg-black/35 px-4 py-3">
              <div class="text-white/50 text-xs tracking-wider uppercase">sharedRecordId</div>
              <div class="mt-2 font-mono text-white/90 break-all text-sm">{{ sharedRecordId }}</div>
              <div v-if="sharedAt" class="mt-2 text-xs text-white/45">sharedAt: {{ sharedAt }}</div>
            </div>
          </div>

          <div class="p-6 border-t border-white/10 bg-white/5 flex justify-end gap-3">
            <button @click="showShareModal = false" class="px-4 py-2 rounded-lg text-sm font-medium text-white/60 hover:text-white transition">关闭</button>
            <button @click="copyShareLink" class="flex items-center gap-2 px-4 py-2 rounded-lg bg-white/10 hover:bg-white/20 text-white text-sm font-medium transition border border-white/10">
              <Copy class="w-4 h-4" />
              复制链接
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <Transition enter-active-class="animate-in fade-in duration-200" leave-active-class="animate-out fade-out duration-150">
      <div v-if="confirmModal" class="fixed inset-0 z-[60] flex items-center justify-center p-4">
        <div class="absolute inset-0 bg-black/80 backdrop-blur-md" @click="closeConfirm"></div>
        <div class="w-full max-w-md bg-neutral-900/90 border border-white/10 rounded-2xl overflow-hidden shadow-2xl relative z-10">
          <div class="px-5 py-4 flex items-center justify-between border-b border-white/10">
            <div class="font-bold text-white">{{ confirmModal.title }}</div>
            <button @click="closeConfirm" class="p-2 rounded-lg hover:bg-white/5 transition-colors">
              <X class="w-5 h-5 text-white/70" />
            </button>
          </div>

          <div class="p-5 text-sm text-white/70 leading-relaxed whitespace-pre-line">
            {{ confirmModal.message }}
          </div>

          <div class="px-5 py-4 bg-black/20 border-t border-white/10 flex items-center justify-end gap-3">
            <button @click="closeConfirm" class="px-5 py-2 rounded-xl bg-white/10 hover:bg-white/20 text-white/80 font-medium transition-colors">
              {{ confirmModal.cancelText }}
            </button>
            <button
              @click="runConfirm"
              :class="[
                'px-5 py-2 rounded-xl font-bold transition-colors',
                confirmModal.kind === 'danger'
                  ? 'bg-red-500/20 hover:bg-red-500/30 text-red-100 border border-red-500/20'
                  : 'bg-white/10 hover:bg-white/20 text-white border border-white/10',
              ]"
            >
              {{ confirmModal.confirmText }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>
