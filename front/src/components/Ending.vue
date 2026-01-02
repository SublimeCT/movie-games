<script setup lang="ts">
import { useStorage } from '@vueuse/core';
import {
  Copy,
  Download,
  FileJson,
  Globe,
  Link as LinkIcon,
  Lock,
  Pencil,
  Share2,
  X,
} from 'lucide-vue-next';
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { getSharedRecordMeta, shareGame } from '../api';
import { useGameState } from '../hooks/useGameState';
import type { Character, Ending, StoryNode } from '../types/movie';

// 使用 hook 获取游戏数据、结局数据和方法
const router = useRouter();

const {
  gameData: data,
  endingData: ending,
  handleRestartPlay,
  handleRemake,
} = useGameState();

const affinityState = useStorage<Record<string, number>>(
  'mg_affinity_state',
  {},
  localStorage,
);

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
  const template = data.value;
  const selected = template?.characters
    ? selectDefaultCharacter(template.characters)
    : null;
  return String(selected?.name || '').trim();
});

const getAffinityBarStyle = (value: number) => {
  const v = Math.max(0, Math.min(100, Math.round(value)));
  const hue = 10 + (v / 100) * 120;
  return {
    width: `${v}%`,
    backgroundImage: `linear-gradient(90deg, hsla(${hue}, 95%, 58%, 0.9), hsla(${
      hue + 18
    }, 95%, 60%, 0.65))`,
  } as const;
};

const affinityRows = computed(() => {
  const characters = data.value?.characters ?? {};
  const protagonist = protagonistName.value;

  const rows = Object.values(characters)
    .map((c) => {
      const name = String(c.name || '').trim();
      if (!name) return null;
      if (protagonist && name === protagonist) return null;

      const raw = affinityState.value[name];
      const value = Number.isFinite(raw) ? Number(raw) : 50;
      const v = Math.max(0, Math.min(100, Math.round(value)));

      return {
        key: String(c.id || name),
        name,
        value: v,
        barStyle: getAffinityBarStyle(v),
      };
    })
    .filter(Boolean) as {
    key: string;
    name: string;
    value: number;
    barStyle: { width: string; backgroundImage: string };
  }[];

  rows.sort((a, b) => a.name.localeCompare(b.name));
  return rows;
});

const isShared = ref(false);
const shareLoading = ref(false);
const showShareModal = ref(false);
const showAnalysisModal = ref(false);

const toast = ref<{ text: string; kind: 'info' | 'success' | 'error' } | null>(
  null,
);
let toastTimer: ReturnType<typeof setTimeout> | null = null;

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

const isOwner = ref(true);
const sharedRecordId = ref<string | null>(null);
const sharedAt = ref<string | null>(null);

/** GLM 的默认请求地址（用于判定“是否被修改”） */
const DEFAULT_GLM_BASE_URL =
  'https://open.bigmodel.cn/api/paas/v4/chat/completions';
/** GLM 的默认模型（用于判定“是否被修改”） */
const DEFAULT_GLM_MODEL = 'glm-4.6v-flash';

const glmBaseUrl = useStorage('mg_glm_base_url', DEFAULT_GLM_BASE_URL);
const glmModel = useStorage('mg_glm_model', DEFAULT_GLM_MODEL);

/**
 * 数据安全锁：当用户自行修改模型配置时，禁用分享与设计功能。
 */
const securityLocked = computed(() => {
  const baseUrlTouched = glmBaseUrl.value.trim() !== DEFAULT_GLM_BASE_URL;
  const modelTouched = glmModel.value.trim() !== DEFAULT_GLM_MODEL;
  return baseUrlTouched || modelTouched;
});

const recordIds = useStorage<string[]>('mg_record_ids', []);
const playEntry = ref<'owner' | 'shared' | 'import'>('owner');

const readPlayEntry = () => {
  const raw = String(sessionStorage.getItem('mg_play_entry') || '').trim();
  if (raw === 'shared') return 'shared' as const;
  if (raw === 'import') return 'import' as const;
  return 'owner' as const;
};

const shareLink = computed(() => {
  if (playEntry.value !== 'owner') return '';
  if (!data.value?.requestId) return '';
  return `${window.location.origin}/play/${data.value.requestId}`;
});

/**
 * 读取分享元信息，避免非创建人在结局页看到分享入口。
 */
const refreshShareMeta = async () => {
  if (playEntry.value === 'import') {
    isOwner.value = false;
    isShared.value = false;
    sharedRecordId.value = null;
    sharedAt.value = null;
    return;
  }

  const requestId = data.value?.requestId;
  if (!requestId) return;

  try {
    const meta = await getSharedRecordMeta(requestId);
    isOwner.value = meta.isOwner;
    isShared.value = meta.shared;
    sharedRecordId.value = meta.sharedRecordId;
    sharedAt.value = meta.sharedAt;
  } catch (e) {
    console.error(e);
    if (playEntry.value === 'shared') {
      isOwner.value = false;
      return;
    }

    isOwner.value = true;
    isShared.value = false;
    sharedRecordId.value = null;
    sharedAt.value = null;
  }
};

watch(
  () => data.value?.requestId,
  () => {
    refreshShareMeta();
  },
);

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

  if (!data.value?.requestId) {
    showToast('此数据不支持在线分享', 'error');
    return;
  }

  shareLoading.value = true;
  try {
    const nextState = !isShared.value;
    const resp = await shareGame(data.value.requestId, nextState);
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
    }
  } catch (e: unknown) {
    console.error('Share failed:', e);
    const msg = e instanceof Error ? e.message : '分享状态更新失败';
    showToast(msg, 'error');
  } finally {
    shareLoading.value = false;
  }
};

const copyShareLink = async () => {
  try {
    await navigator.clipboard.writeText(shareLink.value);
    showToast('链接已复制到剪贴板', 'success');
  } catch (e) {
    console.error('Copy failed:', e);
    showToast('复制失败，请重试', 'error');
  }
};

const cancelShareFromAnalysis = () => {
  showAnalysisModal.value = false;
  void handleShare();
};

const goDesign = () => {
  if (playEntry.value === 'import') {
    sessionStorage.setItem('mg_play_entry', 'import');
    router.push('/design');
    return;
  }

  if (playEntry.value === 'shared' && !isOwner.value) {
    sessionStorage.setItem('mg_play_entry', 'import');
    router.push('/design');
    return;
  }

  if (securityLocked.value) {
    showToast(
      '检测到本地模型配置已被修改（数据安全），已禁用设计功能',
      'error',
    );
    return;
  }

  sessionStorage.setItem('mg_play_entry', 'owner');
  const requestId = data.value?.requestId;
  if (requestId) {
    router.push(`/design?id=${requestId}`);
    return;
  }
  router.push('/design');
};

const resolvedEnding = computed<Ending>(
  () =>
    ending.value ?? {
      type: 'neutral',
      description: 'Game Over',
    },
);

const endingTitle = computed(() => {
  if (resolvedEnding.value.type === 'good') return 'Happy Ending';
  if (resolvedEnding.value.type === 'bad') return 'Bad Ending';
  return 'The End';
});

const stats = computed(() => {
  const nodes = data.value?.nodes ?? {};
  const endings = data.value?.endings ?? {};
  return {
    nodes: Object.keys(nodes).length,
    endings: Object.keys(endings).length,
    characters: Object.keys(data.value?.characters ?? {}).length,
  };
});

const endingDetails = computed(() => {
  const ending = resolvedEnding.value;
  return {
    nodeId: (ending.nodeId || '').trim() || undefined,
    reachedAt: (ending.reachedAt || '').trim() || undefined,
  };
});

/**
 * 结局页的剧情分析信息（用于展示，不参与任何存储）。
 */
const analysis = computed(() => {
  const template = data.value;
  const meta = template?.meta;
  const genre = String(meta?.genre || '').trim();
  const tags = genre
    ? genre
        .split(/[/|,]/g)
        .map((s) => s.trim())
        .filter(Boolean)
        .slice(0, 6)
    : [];

  const language = String(meta?.language || '').trim();
  if (language && !tags.includes(language)) tags.push(language);

  const shareTime = sharedAt.value ? new Date(sharedAt.value) : null;

  return {
    title: template?.title || 'Untitled',
    logline: meta?.logline || '',
    synopsis: meta?.synopsis || '',
    genre,
    tags,
    language,
    runtime: meta?.targetRuntimeMinutes,
    shareStatus: isShared.value ? 'Public' : 'Private',
    sharedAtLabel:
      shareTime && !Number.isNaN(shareTime.getTime())
        ? shareTime.toLocaleString()
        : sharedAt.value || '-',
  };
});

const bgCanvasEl = ref<HTMLCanvasElement | null>(null);
let bgRaf = 0;

const setupBackground = () => {
  const canvas = bgCanvasEl.value;
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  const dpr = Math.max(1, Math.min(2, window.devicePixelRatio || 1));
  const orbs = Array.from({ length: 26 }, (_, i) => {
    const a = (i / 26) * Math.PI * 2;
    return {
      x: 0.5 + 0.36 * Math.cos(a),
      y: 0.5 + 0.36 * Math.sin(a),
      r: 0.08 + (i % 7) * 0.012,
      s: 0.15 + (i % 9) * 0.02,
      p: i * 1.37,
    };
  });

  const resize = () => {
    const rect = canvas.getBoundingClientRect();
    canvas.width = Math.max(1, Math.floor(rect.width * dpr));
    canvas.height = Math.max(1, Math.floor(rect.height * dpr));
  };

  resize();
  const ro = new ResizeObserver(resize);
  ro.observe(canvas);

  const draw = (tMs: number) => {
    const t = tMs / 1000;
    const w = canvas.width;
    const h = canvas.height;

    ctx.clearRect(0, 0, w, h);
    const hue = 260 + Math.sin(t * 0.08) * 35;
    const hue2 = 320 + Math.sin(t * 0.07 + 1.1) * 35;

    const g = ctx.createLinearGradient(0, 0, w, h);
    g.addColorStop(0, `hsla(${hue}, 95%, 55%, 0.55)`);
    g.addColorStop(0.55, `hsla(${hue2}, 95%, 60%, 0.35)`);
    g.addColorStop(1, `hsla(${hue + 40}, 95%, 52%, 0.45)`);
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);

    ctx.globalCompositeOperation = 'lighter';
    for (const o of orbs) {
      const x = (o.x + Math.sin(t * o.s + o.p) * 0.12) * w;
      const y = (o.y + Math.cos(t * o.s * 0.9 + o.p) * 0.12) * h;
      const r = o.r * Math.min(w, h) * (0.9 + 0.2 * Math.sin(t * 0.6 + o.p));
      const rg = ctx.createRadialGradient(x, y, 0, x, y, r);
      rg.addColorStop(0, `hsla(${hue + ((o.p * 40) % 80)}, 98%, 62%, 0.34)`);
      rg.addColorStop(
        0.55,
        `hsla(${hue2 + ((o.p * 50) % 90)}, 98%, 58%, 0.12)`,
      );
      rg.addColorStop(1, 'rgba(0,0,0,0)');
      ctx.fillStyle = rg;
      ctx.beginPath();
      ctx.arc(x, y, r, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.globalCompositeOperation = 'source-over';

    ctx.fillStyle = 'rgba(0,0,0,0.65)';
    ctx.fillRect(0, 0, w, h);

    bgRaf = requestAnimationFrame(draw);
  };

  bgRaf = requestAnimationFrame(draw);

  return () => {
    cancelAnimationFrame(bgRaf);
    ro.disconnect();
  };
};

let stopBg: undefined | (() => void);
onMounted(() => {
  playEntry.value = readPlayEntry();
  refreshShareMeta();

  stopBg = setupBackground();
  window.addEventListener('pointerup', onGlobalPointerUp);
  window.addEventListener('pointercancel', onGlobalPointerUp);
});
onUnmounted(() => {
  if (toastTimer) clearTimeout(toastTimer);
  stopBg?.();
  window.removeEventListener('pointerup', onGlobalPointerUp);
  window.removeEventListener('pointercancel', onGlobalPointerUp);
});

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

/**
 * 当 start 节点存在但没有任何选项时，将节点 1 视为起始节点。
 */
const fallbackStartToOne = computed(() => {
  const nodes = data.value?.nodes;
  if (!nodes?.start) return false;
  if (!nodes['1']) return false;
  const choices = (nodes.start as StoryNode).choices;
  return !Array.isArray(choices) || choices.length === 0;
});

const startNodeId = computed(() => {
  const nodes = data.value?.nodes;
  if (!nodes) return '';
  const keys = Object.keys(nodes);
  if (keys.length === 0) return '';

  if (fallbackStartToOne.value) return '1';
  if (keys.includes('start')) return 'start';
  if (keys.includes('root')) return 'root';
  if (keys.includes('1')) return '1';
  return keys[0];
});

const treeGraph = computed(() => {
  const nodes: Record<string, StoryNode> = data.value?.nodes ?? {};
  const endings = data.value?.endings ?? {};
  const root = startNodeId.value;
  if (!root || !nodes[root]) {
    return {
      nodes: [] as TreeNodeVM[],
      edges: [] as EdgeVM[],
      view: { w: 1000, h: 700 },
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

  // 优化排序：确保子节点尽量靠近父节点
  const layers: string[][] = [];
  for (let i = 0; i <= maxDepth; i++) layers.push([]);

  const placed = new Set<string>();

  // Layer 0
  const rootLayer = byDepth.get(0) || [];
  rootLayer.sort();
  layers[0] = rootLayer;
  rootLayer.forEach((id) => {
    placed.add(id);
  });

  // Layer 1...N
  for (let d = 0; d < maxDepth; d++) {
    const currentLayer = layers[d] ?? [];
    const nextLayerCandidates: string[] = [];

    // 按父节点顺序添加子节点
    for (const pid of currentLayer) {
      const kids = children.get(pid) || [];
      // 按 label 排序，保证同一父节点的子节点有序
      kids.sort((a, b) => (a.label || '').localeCompare(b.label || ''));

      for (const k of kids) {
        if (depth.get(k.to) === d + 1 && !placed.has(k.to)) {
          nextLayerCandidates.push(k.to);
          placed.add(k.to);
        }
      }
    }

    // 添加遗漏的节点（孤立节点或父节点在更上层的）
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

  // 计算最大行数
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
    // 垂直居中每一层
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

const endingFocusId = computed(() => {
  return endingDetails.value.nodeId || '';
});

const highlighted = computed(() => {
  const focus = endingFocusId.value;
  // biome-ignore lint/suspicious/noExplicitAny: D3 graph structure
  const parent = (treeGraph.value as any).parent as
    | Map<string, string>
    | undefined;
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

/**
 * Resets the tree view pan and zoom to default.
 */
const resetView = () => {
  zoom.value = 0.9;
  pan.value = { x: 0, y: 0 };
};

/**
 * Auto-fits the tree visualization to the container.
 * Calculates optimal scale and pan values.
 */
const fitTree = async () => {
  await nextTick();
  const el = treeWrapEl.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  const view = treeGraph.value.view;

  // Subtract padding from container dimensions to avoid edge hugging
  const availableW = rect.width - 40;
  const availableH = rect.height - 40;

  let scale = Math.min(availableW / view.w, availableH / view.h);

  // 移动端检测
  const isMobile = rect.width < 768;

  // 如果缩放比例太小，强制使用可读比例，并靠左对齐，保证让人看清
  if (scale < 0.6) {
    scale = isMobile ? 0.6 : 0.8; // 移动端使用更小的缩放
    zoom.value = scale;
    // 移动端靠左显示，垂直居中
    pan.value = {
      x: isMobile ? 20 : 40,
      y: Math.max(20, (rect.height - view.h * scale) / 2),
    };
  } else {
    // 正常居中适配
    scale = clamp(scale, 0.5, 1.5);
    zoom.value = scale;
    pan.value = {
      x: (rect.width - view.w * scale) / 2,
      y: (rect.height - view.h * scale) / 2,
    };
  }
};

watch(
  () => data.value,
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

const isDragging = ref(false);
const dragStart = ref({ x: 0, y: 0 });

const onPointerDown = (e: PointerEvent) => {
  // Record start position to detect drag vs click
  dragStart.value = { x: e.clientX, y: e.clientY };
  isDragging.value = false;

  // Do NOT capture pointer immediately
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

  // If moved significantly, mark as dragging
  if (
    !isDragging.value &&
    (Math.abs(e.clientX - dragStart.value.x) > 5 ||
      Math.abs(e.clientY - dragStart.value.y) > 5)
  ) {
    isDragging.value = true;
    // Now we know it's a drag, capture the pointer to track movement even outside the element
    (e.currentTarget as HTMLElement)?.setPointerCapture(e.pointerId);
  }

  if (isDragging.value) {
    pan.value = { x: dragging.value.panX + dx, y: dragging.value.panY + dy };
  }
};

const onPointerUp = (e: PointerEvent) => {
  // 释放 pointer capture
  if (dragging.value) {
    (e.currentTarget as HTMLElement)?.releasePointerCapture(e.pointerId);
  }
  dragging.value = null;
  // 延迟重置 isDragging 以便点击事件可以检查它
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

const selectedNodeInfo = computed(() => {
  const id = selectedId.value;
  if (!id) return null;
  const nodes = data.value?.nodes ?? {};
  const endings = data.value?.endings ?? {};
  if (endings[id]) {
    return {
      id,
      kind: 'ending' as const,
      title: id,
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
        : // biome-ignore lint/suspicious/noExplicitAny: Handle legacy object format
          (n.content as any)?.text || '',
    characters: n.characters || [],
    choices: (n.choices || []).map((c) => ({ text: c.text, to: c.nextNodeId })),
  };
});

/**
 * Controls the visibility of the JSON export modal.
 */
const showJsonModal = ref(false);

/**
 * Computes the JSON string representation of the full game data.
 * Used for export and display in the modal.
 * @returns {string} Formatted JSON string
 */
const jsonContent = computed(() => {
  if (!data.value) return '{}';

  const cloned = JSON.parse(JSON.stringify(data.value)) as Record<
    string,
    unknown
  >;
  delete cloned.requestId;

  const endings = cloned.endings;
  if (!endings || typeof endings !== 'object') {
    cloned.endings = {};
  }

  return JSON.stringify(cloned, null, 2);
});

/**
 * Trigger the download of the full game data as a JSON file.
 * Creates a Blob and programmatically clicks a download link.
 */
const downloadJson = () => {
  const blob = new Blob([jsonContent.value], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `movie-game-${data.value?.title || 'export'}-${Date.now()}.json`;
  a.click();
  URL.revokeObjectURL(url);
};

/**
 * Copy the JSON content to clipboard.
 */
const copyJson = async () => {
  try {
    await navigator.clipboard.writeText(jsonContent.value);
    showToast('JSON 已复制到剪贴板', 'success');
  } catch (e) {
    console.error('Copy failed:', e);
    showToast('复制失败，请重试', 'error');
  }
};
</script>

<template>
  <div class="relative min-h-[100dvh] w-full bg-black text-white">
    <canvas ref="bgCanvasEl" class="absolute inset-0 h-full w-full pointer-events-none"></canvas>
    <div class="absolute inset-0 bg-gradient-to-b from-black/15 via-black/55 to-black pointer-events-none"></div>

    <div class="relative z-10 w-full px-6 md:px-10 py-10">
      <div class="w-full">
        <div class="flex flex-col gap-6">
          <div class="flex flex-col md:flex-row md:items-end md:justify-between gap-4">
            <div>
              <div class="text-xs tracking-[0.28em] uppercase text-white/50 font-semibold">Ending</div>
              <h1 class="mt-2 text-4xl md:text-6xl font-black bg-gradient-to-r from-purple-200 via-fuchsia-300 to-cyan-200 bg-clip-text text-transparent">
                {{ endingTitle }}
              </h1>
              <div class="mt-3 flex flex-wrap gap-2">
                <span class="px-3 py-1 rounded-full border border-white/10 bg-white/5 text-xs text-white/70">nodes: {{ stats.nodes }}</span>
                <span class="px-3 py-1 rounded-full border border-white/10 bg-white/5 text-xs text-white/70">endings: {{ stats.endings }}</span>
                <span class="px-3 py-1 rounded-full border border-white/10 bg-white/5 text-xs text-white/70">characters: {{ stats.characters }}</span>
              </div>
            </div>

            <div class="flex flex-col md:flex-row items-stretch md:items-center gap-3 md:gap-4 mt-8 w-full md:w-auto">
              <button
                @click="handleRestartPlay"
                class="group relative inline-flex items-center justify-center px-8 py-3 rounded-xl font-bold text-black bg-white hover:bg-neutral-200 transition-all shadow-[0_0_20px_rgba(255,255,255,0.3)] hover:shadow-[0_0_30px_rgba(255,255,255,0.5)] overflow-hidden w-full md:w-auto"
              >
                <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/50 to-transparent -translate-x-full group-hover:translate-x-full transition-transform duration-700"></div>
                <span class="relative z-10">再次挑战</span>
              </button>

              <button
                @click="handleRemake"
                class="group relative inline-flex items-center justify-center px-6 py-3 rounded-xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md shadow-lg transition-all overflow-hidden w-full md:w-auto"
              >
                <div class="absolute inset-0 bg-white/5 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
                <span class="relative z-10">重新生成</span>
              </button>

              <div class="hidden md:block h-8 w-px bg-white/10 mx-2"></div>

              <!-- Accessibility Info -->
              <div class="flex items-center justify-center md:justify-start gap-2 px-3 py-1.5 rounded-lg border border-white/5 bg-white/5 backdrop-blur-sm select-none w-full md:w-auto">
                <div :class="['w-2 h-2 rounded-full animate-pulse', isShared ? 'bg-green-500 shadow-[0_0_8px_rgba(34,197,94,0.6)]' : 'bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.6)]']"></div>
                <span class="text-xs font-mono text-white/60 uppercase tracking-wider">
                  {{ isShared ? 'Public' : 'Private' }}
                </span>
              </div>

              <button
                v-if="playEntry === 'import' || (isOwner && playEntry === 'owner') || (!isOwner && playEntry === 'shared')"
                @click="goDesign"
                :disabled="(playEntry === 'owner' && securityLocked)"
                class="group relative inline-flex items-center justify-center px-4 py-3 rounded-xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md shadow-[0_0_25px_rgba(168,85,247,0.14)] transition-all gap-2 overflow-hidden w-full md:w-auto disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <div class="absolute inset-0 bg-white/10 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
                <Pencil class="w-4 h-4 relative z-10" />
                <span class="relative z-10">进入设计</span>
              </button>

              <!-- Share Button (仅创建人可见) -->
              <button
                v-if="isOwner && playEntry === 'owner'"
                @click="handleShare"
                :disabled="shareLoading || securityLocked"
                class="group relative inline-flex items-center justify-center px-4 py-3 rounded-xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md shadow-[0_0_25px_rgba(34,211,238,0.14)] transition-all gap-2 overflow-hidden disabled:opacity-50 disabled:cursor-not-allowed w-full md:w-auto"
              >
                <div class="absolute inset-0 bg-white/10 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
                <Share2 v-if="!isShared" class="w-4 h-4 relative z-10" />
                <Lock v-else class="w-4 h-4 relative z-10" />
                <span class="relative z-10">{{ shareLoading ? '处理中...' : (isShared ? '取消分享' : '分享剧情') }}</span>
              </button>

              <button
                @click="showJsonModal = true"
                class="group relative inline-flex items-center justify-center px-4 py-3 rounded-xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md shadow-[0_0_25px_rgba(34,211,238,0.14)] transition-all gap-2 overflow-hidden w-full md:w-auto"
              >
                <div class="absolute inset-0 bg-white/10 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
                <FileJson class="w-4 h-4 relative z-10" />
                <span class="relative z-10">导出</span>
              </button>
            </div>
          </div>

          <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <section class="rounded-2xl border border-white/10 bg-black/35 backdrop-blur-xl p-6 shadow-2xl">
              <div class="text-xs tracking-[0.24em] uppercase text-white/50 font-semibold">结局说明</div>
              <div class="mt-4 text-lg md:text-xl leading-relaxed text-white/85">
                {{ resolvedEnding.description }}
              </div>

              <div class="mt-6 grid grid-cols-1 md:grid-cols-2 gap-3 text-sm">
                <div class="rounded-xl border border-white/10 bg-white/5 px-4 py-3">
                  <div class="text-white/50 text-xs tracking-wider uppercase">type</div>
                  <div class="mt-1 font-semibold text-white/90">{{ resolvedEnding.type }}</div>
                </div>
                <!-- endingKey removed -->
                <div class="rounded-xl border border-white/10 bg-white/5 px-4 py-3">
                  <div class="text-white/50 text-xs tracking-wider uppercase">nodeId</div>
                  <div class="mt-1 font-mono text-white/90 break-all">{{ endingDetails.nodeId || '-' }}</div>
                </div>
                <div class="rounded-xl border border-white/10 bg-white/5 px-4 py-3">
                  <div class="text-white/50 text-xs tracking-wider uppercase">reachedAt</div>
                  <div class="mt-1 font-mono text-white/90 break-all">{{ endingDetails.reachedAt || '-' }}</div>
                </div>
              </div>

              <div class="mt-6">
                <div class="text-xs tracking-[0.24em] uppercase text-white/50 font-semibold">好感度</div>

                <div v-if="affinityRows.length === 0" class="mt-3 text-sm text-white/50">
                  暂无可展示的角色好感度
                </div>

                <div v-else class="mt-3 space-y-2">
                  <div
                    v-for="row in affinityRows"
                    :key="row.key"
                    class="rounded-xl border border-white/10 bg-white/5 px-4 py-3"
                  >
                    <div class="flex items-center justify-between gap-3">
                      <div class="text-sm font-semibold text-white/90 truncate">{{ row.name }}</div>
                      <div class="text-xs font-mono text-white/70">{{ row.value }}%</div>
                    </div>
                    <div class="mt-2 h-2 rounded-full bg-white/10 overflow-hidden">
                      <div class="h-full rounded-full" :style="row.barStyle"></div>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Endings Map Removed -->
            </section>

            <section class="rounded-2xl border border-white/10 bg-black/35 backdrop-blur-xl p-6 shadow-2xl overflow-hidden">
              <div class="flex items-center justify-between gap-4">
                <div class="text-xs tracking-[0.24em] uppercase text-white/50 font-semibold">剧情树</div>
              <div class="flex items-center gap-2">
                <button class="px-3 py-1 rounded-lg border border-white/10 bg-white/5 text-xs text-white/70 hover:bg-white/10 transition" @click="fitTree">适配视图</button>
                <button class="px-3 py-1 rounded-lg border border-white/10 bg-white/5 text-xs text-white/70 hover:bg-white/10 transition" @click="resetView">重置</button>
              </div>
              </div>

              <div ref="treeWrapEl" class="mt-4 rounded-xl border border-white/10 bg-black/55 overflow-hidden h-[640px] md:h-[720px] relative"
                   style="touch-action: none; user-select: none;"
                   @wheel="onWheel"
                   @pointerdown="onPointerDown"
                   @pointermove="onPointerMove"
                   @pointerup="onPointerUp"
                   @pointercancel="onPointerUp">
                <!-- Transform Container -->
                <div
                  class="absolute top-0 left-0 w-full h-full origin-top-left will-change-transform"
                  :style="{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})` }"
                >
                  <!-- Edges Layer (SVG) -->
                  <svg class="absolute inset-0 overflow-visible pointer-events-none z-0" width="1" height="1">
                    <defs>
                      <linearGradient id="edge-gradient" gradientUnits="userSpaceOnUse" x1="0%" y1="0%" x2="100%" y2="0%">
                        <stop offset="0%" stop-color="#9333ea" stop-opacity="0.3" />
                        <stop offset="100%" stop-color="#22d3ee" stop-opacity="0.3" />
                      </linearGradient>
                      <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto">
                        <polygon points="0 0, 10 3.5, 0 7" fill="#64748b" opacity="0.5" />
                      </marker>
                    </defs>

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
                        :stroke-width="(highlighted.has(e.from) && highlighted.has(e.to)) ? (3 / zoom) : (1.5 / zoom)"
                        :stroke-opacity="(highlighted.has(e.from) && highlighted.has(e.to)) ? 0.8 : 0.3"
                        fill="none"
                        class="transition-all duration-500"
                        marker-end="url(#arrowhead)"
                      />
                    </g>
                  </svg>

                  <!-- Nodes Layer (HTML) -->
                  <div
                    v-for="n in treeGraph.nodes"
                    :key="n.id"
                    class="absolute z-10"
                    :style="{
                      left: `${n.x}px`,
                      top: `${n.y}px`,
                      width: `${n.w}px`,
                      height: `${n.h}px`
                    }"
                  >
                    <div
                      data-node
                      :data-node-id="n.id"
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
                      <div class="flex items-center justify-between mb-1 pointer-events-none">
                        <span :class="['text-[10px] font-mono uppercase tracking-wider', n.kind === 'ending' ? 'text-cyan-400' : 'text-purple-400']">
                          {{ n.kind === 'ending' ? 'ENDING' : 'NODE' }}
                        </span>
                        <div v-if="highlighted.has(n.id)" class="w-1.5 h-1.5 rounded-full bg-green-400 animate-pulse shadow-[0_0_5px_rgba(74,222,128,0.8)]"></div>
                      </div>
                      <div class="text-xs font-bold text-white/90 truncate font-mono pointer-events-none">{{ n.label }}</div>
                      
                      <!-- Hover Glow -->
                      <div class="absolute inset-0 rounded-xl bg-gradient-to-r from-white/0 via-white/5 to-white/0 opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none"></div>
                    </div>
                  </div>
                </div>

                <div 
                  v-if="selectedNodeInfo" 
                  class="absolute right-4 top-4 w-[320px] max-w-[90%] rounded-2xl border border-white/10 bg-black/90 backdrop-blur-xl p-5 shadow-2xl ring-1 ring-white/5 z-20"
                  @pointerdown.stop
                  @mousedown.stop
                  @click.stop
                >
                  <div class="flex items-center justify-between gap-2 mb-4">
                    <div class="text-xs tracking-[0.22em] uppercase text-white/50 font-semibold">{{ selectedNodeInfo.kind === 'ending' ? 'Ending' : 'Node' }}</div>
                    <button 
                      class="text-white/40 hover:text-white p-1 rounded-full hover:bg-white/10 transition-colors" 
                      @click.stop="selectedId = ''"
                    >
                      <X class="w-4 h-4" />
                    </button>
                  </div>

                  <div class="font-mono text-white/90 break-all text-sm font-bold border-b border-white/10 pb-3 mb-3">{{ selectedNodeInfo.id }}</div>

                  <template v-if="selectedNodeInfo.kind === 'ending'">
                    <div class="mt-3 text-sm text-white/80 leading-relaxed">{{ selectedNodeInfo.description }}</div>
                    <div class="mt-3 text-xs text-white/55">type: {{ selectedNodeInfo.type }}</div>
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
                  </template>
                </div>
              </div>
            </section>
          </div>
        </div>
      </div>
    </div>

    <!-- JSON Export Modal -->
    <Teleport to="body">
      <div v-if="showJsonModal" class="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm" @click.self="showJsonModal = false">
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
    </Teleport>

    <!-- Analysis Modal -->
    <Teleport to="body">
      <div v-if="showAnalysisModal" class="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm" @click.self="showAnalysisModal = false">
        <div class="relative w-full max-w-3xl rounded-2xl border border-white/10 bg-[#0a0a0a] shadow-2xl overflow-hidden flex flex-col">
          <div class="flex items-center justify-between px-6 py-4 border-b border-white/10 bg-white/5">
            <div>
              <div class="text-xs tracking-[0.28em] uppercase text-white/50 font-semibold">Analysis</div>
              <div class="mt-1 text-xl md:text-2xl font-black text-white/90">剧情分析</div>
            </div>
            <button @click="showAnalysisModal = false" class="text-white/50 hover:text-white transition">
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="p-6 md:p-8 space-y-6 overflow-y-auto">
            <div class="rounded-2xl border border-white/10 bg-white/5 p-5">
              <div class="text-xs tracking-[0.24em] uppercase text-white/50 font-semibold">Title</div>
              <div class="mt-3 text-2xl md:text-3xl font-black text-white/90">{{ analysis.title }}</div>
              <div class="mt-3 flex flex-wrap gap-2">
                <span
                  v-for="tag in analysis.tags"
                  :key="tag"
                  class="px-3 py-1 rounded-full border border-white/10 bg-black/35 text-xs text-white/70 hover:bg-white/10 transition"
                >
                  {{ tag }}
                </span>
              </div>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
              <div class="rounded-2xl border border-white/10 bg-white/5 p-5">
                <div class="text-xs tracking-[0.24em] uppercase text-white/50 font-semibold">Logline</div>
                <div class="mt-3 text-sm md:text-base leading-relaxed text-white/80">
                  {{ analysis.logline || '（暂无 Logline）' }}
                </div>
              </div>
              <div class="rounded-2xl border border-white/10 bg-white/5 p-5">
                <div class="text-xs tracking-[0.24em] uppercase text-white/50 font-semibold">Synopsis</div>
                <div class="mt-3 text-sm md:text-base leading-relaxed text-white/80">
                  {{ analysis.synopsis || '（暂无剧情简介）' }}
                </div>
              </div>
            </div>

            <div class="rounded-2xl border border-white/10 bg-white/5 p-5">
              <div class="text-xs tracking-[0.24em] uppercase text-white/50 font-semibold">Metrics</div>
              <div class="mt-4 grid grid-cols-1 md:grid-cols-4 gap-3 text-sm">
                <div class="rounded-xl border border-white/10 bg-black/35 px-4 py-3">
                  <div class="text-white/50 text-xs tracking-wider uppercase">nodes</div>
                  <div class="mt-1 font-mono text-white/90">{{ stats.nodes }}</div>
                </div>
                <div class="rounded-xl border border-white/10 bg-black/35 px-4 py-3">
                  <div class="text-white/50 text-xs tracking-wider uppercase">endings</div>
                  <div class="mt-1 font-mono text-white/90">{{ stats.endings }}</div>
                </div>
                <div class="rounded-xl border border-white/10 bg-black/35 px-4 py-3">
                  <div class="text-white/50 text-xs tracking-wider uppercase">characters</div>
                  <div class="mt-1 font-mono text-white/90">{{ stats.characters }}</div>
                </div>
                <div class="rounded-xl border border-white/10 bg-black/35 px-4 py-3">
                  <div class="text-white/50 text-xs tracking-wider uppercase">runtime</div>
                  <div class="mt-1 font-mono text-white/90">{{ analysis.runtime ?? '-' }}</div>
                </div>
              </div>
            </div>

            <div class="rounded-2xl border border-white/10 bg-white/5 p-5">
              <div class="text-xs tracking-[0.24em] uppercase text-white/50 font-semibold">Share</div>
              <div class="mt-4 grid grid-cols-1 md:grid-cols-2 gap-3 text-sm">
                <div class="rounded-xl border border-white/10 bg-black/35 px-4 py-3">
                  <div class="text-white/50 text-xs tracking-wider uppercase">status</div>
                  <div class="mt-1 font-mono text-white/90">{{ analysis.shareStatus }}</div>
                </div>
                <div class="rounded-xl border border-white/10 bg-black/35 px-4 py-3">
                  <div class="text-white/50 text-xs tracking-wider uppercase">sharedAt</div>
                  <div class="mt-1 font-mono text-white/90 break-all">{{ analysis.sharedAtLabel }}</div>
                </div>
                <div class="rounded-xl border border-white/10 bg-black/35 px-4 py-3 md:col-span-2">
                  <div class="text-white/50 text-xs tracking-wider uppercase">requestId</div>
                  <div class="mt-1 font-mono text-white/90 break-all">{{ data?.requestId || '-' }}</div>
                </div>
                <div v-if="isOwner && playEntry === 'owner' && sharedRecordId" class="rounded-xl border border-white/10 bg-black/35 px-4 py-3 md:col-span-2">
                  <div class="text-white/50 text-xs tracking-wider uppercase">sharedRecordId</div>
                  <div class="mt-1 font-mono text-white/90 break-all">{{ sharedRecordId }}</div>
                </div>
              </div>

              <div class="mt-5 flex flex-col md:flex-row gap-3">
                <button
                  v-if="shareLink && playEntry === 'owner'"
                  @click="copyShareLink"
                  class="group relative inline-flex items-center justify-center px-4 py-3 rounded-xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md shadow-[0_0_25px_rgba(34,211,238,0.14)] transition-all gap-2 overflow-hidden w-full md:w-auto"
                >
                  <div class="absolute inset-0 bg-white/10 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
                  <Copy class="w-4 h-4 relative z-10" />
                  <span class="relative z-10">复制分享链接</span>
                </button>

                <button
                  v-if="isOwner && playEntry === 'owner' && isShared"
                  @click="cancelShareFromAnalysis"
                  class="group relative inline-flex items-center justify-center px-4 py-3 rounded-xl font-bold text-white/90 border border-red-500/20 bg-red-500/10 hover:bg-red-500/15 backdrop-blur-md transition-all gap-2 overflow-hidden w-full md:w-auto"
                >
                  <div class="absolute inset-0 bg-white/10 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
                  <Lock class="w-4 h-4 relative z-10" />
                  <span class="relative z-10">取消分享</span>
                </button>

                <button
                  @click="showAnalysisModal = false"
                  class="group relative inline-flex items-center justify-center px-4 py-3 rounded-xl font-bold text-black bg-white hover:bg-neutral-200 transition-all overflow-hidden w-full md:w-auto"
                >
                  <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/60 to-transparent -translate-x-full group-hover:translate-x-full transition-transform duration-700"></div>
                  <span class="relative z-10">关闭</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Share Modal -->
    <Teleport to="body">
      <div v-if="showShareModal" class="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm" @click.self="showShareModal = false">
        <div class="relative w-full max-w-lg rounded-2xl border border-white/10 bg-[#0a0a0a] shadow-2xl overflow-hidden flex flex-col">
          <div class="flex items-center justify-between px-6 py-4 border-b border-white/10 bg-white/5">
            <div class="flex items-center gap-2">
              <Globe class="w-5 h-5 text-green-400" />
              <span class="font-bold text-white/90">剧情已分享</span>
            </div>
            <button @click="showShareModal = false" class="text-white/50 hover:text-white transition">
              <X class="w-5 h-5" />
            </button>
          </div>
          
          <div class="p-6">
            <p class="text-white/70 text-sm mb-4 leading-relaxed">
              您的剧情已成功设置为公开访问。任何拥有链接的人都可以体验此剧情。
            </p>
            
            <div class="flex items-center gap-2 p-3 rounded-xl border border-white/10 bg-black/50 mb-4">
              <LinkIcon class="w-4 h-4 text-white/40 flex-shrink-0" />
              <a :href="shareLink" target="_blank" class="flex-1 font-mono text-xs text-cyan-400 hover:text-cyan-300 underline truncate">{{ shareLink }}</a>
              <button 
                @click="copyShareLink"
                class="px-3 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-xs font-bold text-white transition whitespace-nowrap"
              >
                复制链接
              </button>
            </div>
            
            <div class="flex items-center gap-3 p-3 rounded-xl border border-yellow-500/20 bg-yellow-500/5">
               <Lock class="w-4 h-4 text-yellow-500/80 flex-shrink-0" />
               <div class="text-xs text-yellow-200/80 leading-relaxed">
                 再次点击页面上的"取消分享"按钮可随时撤回访问权限。撤回后，此链接将立即失效。
               </div>
            </div>
          </div>
          
          <div class="p-6 border-t border-white/10 bg-white/5 flex justify-end">
            <button
              @click="showShareModal = false"
              class="px-5 py-2.5 rounded-xl font-bold text-black bg-white hover:bg-white/90 transition shadow-lg shadow-white/10"
            >
              完成
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: rgba(255, 255, 255, 0.05);
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 2px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.3);
}
</style>
