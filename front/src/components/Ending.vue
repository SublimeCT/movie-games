<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { X, Download, FileJson, Share2, Globe, Link as LinkIcon, Lock } from 'lucide-vue-next';
import type { Ending, MovieTemplate, StoryNode } from '../types/movie';
import { shareGame } from '../api';

const props = defineProps<{
  data?: MovieTemplate | null;
  ending?: Ending | null;
}>();

const emit = defineEmits<{
  (e: 'restartPlay'): void;
  (e: 'remake'): void;
}>();

const isShared = ref(false);
const shareLoading = ref(false);
const showShareModal = ref(false);

const shareLink = computed(() => {
  if (!props.data?.requestId) return '';
  return `${window.location.origin}/play/${props.data.requestId}`;
});

const handleShare = async () => {
  if (!props.data?.requestId) {
    alert('此数据为本地导入或旧版数据，不支持在线分享');
    return;
  }
  
  shareLoading.value = true;
  try {
    const nextState = !isShared.value;
    await shareGame(props.data.requestId, nextState);
    isShared.value = nextState;
    if (nextState) {
      showShareModal.value = true;
    }
  } catch (e) {
    console.error('Share failed:', e);
    // @ts-ignore
    const msg = e.message || '分享状态更新失败';
    alert(msg);
  } finally {
    shareLoading.value = false;
  }
};

const copyShareLink = async () => {
  try {
    await navigator.clipboard.writeText(shareLink.value);
    alert('链接已复制到剪贴板');
  } catch (e) {
    console.error('Copy failed:', e);
  }
};

const resolvedEnding = computed<Ending>(
  () =>
    props.ending ?? {
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
  const nodes = props.data?.nodes ?? {};
  const endings = props.data?.endings ?? {};
  return {
    nodes: Object.keys(nodes).length,
    endings: Object.keys(endings).length,
    characters: Object.keys(props.data?.characters ?? {}).length,
  };
});

const endingDetails = computed(() => {
  const ending = resolvedEnding.value;
  return {
    nodeId: (ending.nodeId || '').trim() || undefined,
    reachedAt: (ending.reachedAt || '').trim() || undefined,
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
  stopBg = setupBackground();
  window.addEventListener('pointerup', onGlobalPointerUp);
  window.addEventListener('pointercancel', onGlobalPointerUp);
});
onUnmounted(() => {
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

const startNodeId = computed(() => {
  const nodes = props.data?.nodes;
  if (!nodes) return '';
  const keys = Object.keys(nodes);
  if (keys.length === 0) return '';
  if (keys.includes('start')) return 'start';
  if (keys.includes('root')) return 'root';
  if (keys.includes('1')) return '1';
  return keys[0];
});

const treeGraph = computed(() => {
  const nodes: Record<string, StoryNode> = props.data?.nodes ?? {};
  const endings = props.data?.endings ?? {};
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
  rootLayer.forEach(id => placed.add(id));

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
    
    layers[d+1] = nextLayerCandidates;
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

  return { nodes: nodeVMs, edges, view: { w: totalW, h: totalH + padY * 2 }, parent };
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

  // 如果缩放比例太小，强制使用可读比例，并靠左对齐，保证让人看清
  if (scale < 0.6) {
    scale = 0.8;
    zoom.value = scale;
    // 靠左显示，垂直居中
    pan.value = {
      x: 40,
      y: (rect.height - view.h * scale) / 2,
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
  () => props.data,
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

  // We don't capture immediately to allow click events to propagate naturally
  // if no movement occurs. But we need to track movement.
  // Actually, for smooth dragging, we usually want capture.
  // Let's capture, but handle click vs drag manually.
  // (e.currentTarget as HTMLElement | null)?.setPointerCapture?.(e.pointerId);
  
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
  if (!isDragging.value && (Math.abs(e.clientX - dragStart.value.x) > 5 || Math.abs(e.clientY - dragStart.value.y) > 5)) {
    isDragging.value = true;
  }
  
  pan.value = { x: dragging.value.panX + dx, y: dragging.value.panY + dy };
};

const onPointerUp = () => {
  dragging.value = null;
  // We don't reset isDragging here immediately if we need to check it in click handlers
  // But click handlers usually fire after pointerup.
  // Let's rely on isDragging in the click handler.
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
  const nodes = props.data?.nodes ?? {};
  const endings = props.data?.endings ?? {};
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
  // 导出完整剧情信息，可以直接在主页导入
  if (!props.data) return '{}';
  // 确保包含所有必要字段，特别是 requestId（如果有）
  // 深拷贝以避免副作用，虽然 JSON.stringify 本身不会修改原对象
  const dataToExport = { ...props.data };
  return JSON.stringify(dataToExport, null, 2);
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
  a.download = `movie-game-${props.data?.title || 'export'}-${Date.now()}.json`;
  a.click();
  URL.revokeObjectURL(url);
};
</script>

<template>
  <div class="relative h-screen w-full overflow-hidden bg-black text-white">
    <canvas ref="bgCanvasEl" class="absolute inset-0 h-full w-full pointer-events-none"></canvas>
    <div class="absolute inset-0 bg-gradient-to-b from-black/15 via-black/55 to-black pointer-events-none"></div>

    <div class="relative z-10 h-full w-full overflow-y-auto px-6 md:px-10 py-10">
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
                @click="emit('restartPlay')"
                class="group relative inline-flex items-center justify-center px-8 py-3 rounded-xl font-bold text-black bg-white hover:bg-neutral-200 transition-all shadow-[0_0_20px_rgba(255,255,255,0.3)] hover:shadow-[0_0_30px_rgba(255,255,255,0.5)] overflow-hidden w-full md:w-auto"
              >
                <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/50 to-transparent -translate-x-full group-hover:translate-x-full transition-transform duration-700"></div>
                <span class="relative z-10">再次挑战</span>
              </button>
              
              <button
                @click="emit('remake')"
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

              <!-- Share Button -->
              <button
                @click="handleShare"
                :disabled="shareLoading"
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

              <div ref="treeWrapEl" class="mt-4 rounded-xl border border-white/10 bg-black/55 overflow-hidden h-[640px] md:h-[720px] relative touch-none" @wheel="onWheel" @pointerdown="onPointerDown" @pointermove="onPointerMove" @pointerup="onPointerUp" @pointercancel="onPointerUp">
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
                    <!-- Edges -->
                    <g>
                      <path
                        v-for="(e, idx) in treeGraph.edges"
                        :key="idx"
                        :d="(() => {
                          const a = treeGraph.nodes.find(n => n.id === e.from);
                          const b = treeGraph.nodes.find(n => n.id === e.to);
                          if (!a || !b) return '';
                          
                          // Connect from right of A to left of B
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

                    <!-- Nodes -->
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
                          <div class="text-xs font-bold text-white/90 truncate font-mono">
                            {{ n.label }}
                          </div>
                          
                          <!-- Hover Glow -->
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
            <button @click="downloadJson" class="flex items-center gap-2 px-4 py-2 rounded-lg bg-white/10 hover:bg-white/20 text-white text-sm font-medium transition border border-white/10">
              <Download class="w-4 h-4" />
              下载 JSON
            </button>
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
