<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import type { Ending, MovieTemplate, StoryNode } from '../types/movie';

const props = defineProps<{
  data?: MovieTemplate | null
  ending?: Ending | null
}>();

const emit = defineEmits<{
  (e: 'restartPlay'): void
  (e: 'remake'): void
}>();

const resolvedEnding = computed<Ending>(() =>
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
  const endingKey = (ending.endingKey || '').trim();
  const fromMap = endingKey ? props.data?.endings?.[endingKey] : undefined;
  return {
    endingKey: endingKey || undefined,
    nodeId: (ending.nodeId || '').trim() || undefined,
    reachedAt: (ending.reachedAt || '').trim() || undefined,
    fromMap,
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
      rg.addColorStop(0, `hsla(${hue + (o.p * 40) % 80}, 98%, 62%, 0.34)`);
      rg.addColorStop(0.55, `hsla(${hue2 + (o.p * 50) % 90}, 98%, 58%, 0.12)`);
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
  if (keys.includes('n_start')) return 'n_start';
  if (keys.includes('root')) return 'root';
  if (keys.includes('1')) return '1';
  return keys[0];
});

const treeGraph = computed(() => {
  const nodes: Record<string, StoryNode> = props.data?.nodes ?? {};
  const endings = props.data?.endings ?? {};
  const root = startNodeId.value;
  if (!root || !nodes[root]) {
    return { nodes: [] as TreeNodeVM[], edges: [] as EdgeVM[], view: { w: 1000, h: 700 } };
  }

  const knownEndingKeys = new Set(Object.keys(endings));
  const children = new Map<string, { to: string; label?: string }[]>();

  for (const [id, n] of Object.entries(nodes)) {
    const list: { to: string; label?: string }[] = [];
    const endingKey = (n.endingKey || '').trim();
    if (endingKey && knownEndingKeys.has(endingKey)) {
      list.push({ to: endingKey, label: 'ending' });
    }
    for (const c of n.choices || []) {
      const to = (c.nextNodeId || '').trim();
      if (!to) continue;
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
    byDepth.get(d)!.push(id);
  }

  const maxDepth = Math.max(...Array.from(byDepth.keys()));
  for (const ids of byDepth.values()) {
    ids.sort((a, b) => a.localeCompare(b));
  }

  const xStep = 240;
  const yStep = 110;
  const padX = 140;
  const padY = 120;

  const pos = new Map<string, { x: number; y: number }>();
  let maxRow = 0;
  for (let d = 0; d <= maxDepth; d += 1) {
    const ids = byDepth.get(d) ?? [];
    maxRow = Math.max(maxRow, ids.length);
    for (let i = 0; i < ids.length; i += 1) {
      const id = ids[i];
      if (!id) continue;
      pos.set(id, {
        x: padX + d * xStep,
        y: padY + i * yStep,
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
    });
  }

  const w = padX * 2 + (maxDepth + 1) * xStep;
  const h = padY * 2 + Math.max(1, maxRow) * yStep;

  return { nodes: nodeVMs, edges, view: { w, h }, parent };
});

const selectedId = ref<string>('');
const hoveredId = ref<string>('');
const treeWrapEl = ref<HTMLDivElement | null>(null);
const pan = ref({ x: 0, y: 0 });
const zoom = ref(0.9);
const dragging = ref<null | { x: number; y: number; panX: number; panY: number }>(null);

const endingFocusId = computed(() => {
  if (endingDetails.value.endingKey) return endingDetails.value.endingKey;
  if (endingDetails.value.nodeId) return endingDetails.value.nodeId;
  return '';
});

const highlighted = computed(() => {
  const focus = endingFocusId.value;
  const parent = (treeGraph.value as any).parent as Map<string, string> | undefined;
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

const clamp = (v: number, min: number, max: number) => Math.max(min, Math.min(max, v));

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
  const scale = clamp(Math.min(rect.width / view.w, rect.height / view.h) * 0.92, 0.45, 1.2);
  zoom.value = scale;
  pan.value = {
    x: (rect.width - view.w * scale) / 2,
    y: (rect.height - view.h * scale) / 2,
  };
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
  const nextZoom = clamp(prevZoom * factor, 0.4, 2.2);

  const wx = (mx - pan.value.x) / prevZoom;
  const wy = (my - pan.value.y) / prevZoom;
  zoom.value = nextZoom;
  pan.value = {
    x: mx - wx * nextZoom,
    y: my - wy * nextZoom,
  };
};

const onPointerDown = (e: PointerEvent) => {
  if ((e.target as HTMLElement | null)?.closest?.('[data-node]')) return;
  (e.currentTarget as HTMLElement | null)?.setPointerCapture?.(e.pointerId);
  dragging.value = { x: e.clientX, y: e.clientY, panX: pan.value.x, panY: pan.value.y };
};

const onPointerMove = (e: PointerEvent) => {
  if (!dragging.value) return;
  const dx = e.clientX - dragging.value.x;
  const dy = e.clientY - dragging.value.y;
  pan.value = { x: dragging.value.panX + dx, y: dragging.value.panY + dy };
};

const onPointerUp = () => {
  dragging.value = null;
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
    content: n.content?.text || '',
    endingKey: (n.endingKey || '').trim() || undefined,
    characters: n.characters || [],
    choices: (n.choices || []).map((c) => ({ text: c.text, to: c.nextNodeId })),
  };
});
</script>

<template>
  <div class="relative h-screen w-full overflow-hidden bg-black text-white">
    <canvas ref="bgCanvasEl" class="absolute inset-0 h-full w-full"></canvas>
    <div class="absolute inset-0 bg-gradient-to-b from-black/15 via-black/55 to-black"></div>

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

            <div class="flex gap-3">
              <button
                @click="emit('restartPlay')"
                class="relative inline-flex items-center justify-center px-6 py-3 rounded-xl font-bold text-white border border-white/10 bg-gradient-to-r from-purple-600/90 via-fuchsia-600/90 to-cyan-500/85 hover:from-purple-500 hover:via-fuchsia-500 hover:to-cyan-400 shadow-[0_0_30px_rgba(217,70,239,0.22)] transition-all"
              >
                重新开始
              </button>
              <button
                @click="emit('remake')"
                class="inline-flex items-center justify-center px-6 py-3 rounded-xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md shadow-[0_0_25px_rgba(34,211,238,0.14)] transition-all"
              >
                重新制作
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
                <div class="rounded-xl border border-white/10 bg-white/5 px-4 py-3">
                  <div class="text-white/50 text-xs tracking-wider uppercase">endingKey</div>
                  <div class="mt-1 font-mono text-white/90 break-all">{{ endingDetails.endingKey || '-' }}</div>
                </div>
                <div class="rounded-xl border border-white/10 bg-white/5 px-4 py-3">
                  <div class="text-white/50 text-xs tracking-wider uppercase">nodeId</div>
                  <div class="mt-1 font-mono text-white/90 break-all">{{ endingDetails.nodeId || '-' }}</div>
                </div>
                <div class="rounded-xl border border-white/10 bg-white/5 px-4 py-3">
                  <div class="text-white/50 text-xs tracking-wider uppercase">reachedAt</div>
                  <div class="mt-1 font-mono text-white/90 break-all">{{ endingDetails.reachedAt || '-' }}</div>
                </div>
              </div>

              <div v-if="endingDetails.fromMap" class="mt-6 rounded-2xl border border-white/10 bg-white/5 p-5">
                <div class="text-xs tracking-[0.24em] uppercase text-white/50 font-semibold">endings 映射</div>
                <div class="mt-3 text-sm text-white/80 leading-relaxed">{{ endingDetails.fromMap.description }}</div>
              </div>
            </section>

            <section class="rounded-2xl border border-white/10 bg-black/35 backdrop-blur-xl p-6 shadow-2xl overflow-hidden">
              <div class="flex items-center justify-between gap-4">
                <div class="text-xs tracking-[0.24em] uppercase text-white/50 font-semibold">剧情树</div>
              <div class="flex items-center gap-2">
                <button class="px-3 py-1 rounded-lg border border-white/10 bg-white/5 text-xs text-white/70 hover:bg-white/10 transition" @click="fitTree">适配视图</button>
                <button class="px-3 py-1 rounded-lg border border-white/10 bg-white/5 text-xs text-white/70 hover:bg-white/10 transition" @click="resetView">重置</button>
              </div>
              </div>

              <div ref="treeWrapEl" class="mt-4 rounded-xl border border-white/10 bg-black/55 overflow-hidden h-[640px] md:h-[720px] relative" @wheel="onWheel" @pointerdown="onPointerDown" @pointermove="onPointerMove" @pointerup="onPointerUp" @pointercancel="onPointerUp">
                <svg :viewBox="`0 0 ${treeGraph.view.w} ${treeGraph.view.h}`" class="absolute inset-0 w-full h-full select-none" :style="{ cursor: dragging ? 'grabbing' : 'grab' }">
                  <g :transform="`translate(${pan.x} ${pan.y}) scale(${zoom})`">
                    <g>
                      <path
                        v-for="(e, idx) in treeGraph.edges"
                        :key="idx"
                        :d="(() => {
                          const a = treeGraph.nodes.find(n => n.id === e.from);
                          const b = treeGraph.nodes.find(n => n.id === e.to);
                          if (!a || !b) return '';
                          const mx = (a.x + b.x) / 2;
                          return `M ${a.x} ${a.y} C ${mx} ${a.y}, ${mx} ${b.y}, ${b.x} ${b.y}`;
                        })()"
                        :stroke="(highlighted.has(e.from) && highlighted.has(e.to)) ? 'rgba(217,70,239,0.32)' : 'rgba(255,255,255,0.14)'"
                        stroke-width="2"
                        fill="none"
                      />
                    </g>

                    <g>
                      <g v-for="n in treeGraph.nodes" :key="n.id" data-node @click.stop="selectedId = n.id" @pointerenter="hoveredId = n.id" @pointerleave="hoveredId = ''" :transform="`translate(${n.x} ${n.y})`" class="cursor-pointer">
                        <circle
                          :r="(n.kind === 'ending' ? 26 : 22) + ((hoveredId === n.id || selectedId === n.id) ? 5 : 0)"
                          :fill="highlighted.has(n.id) ? 'rgba(217,70,239,0.45)' : (n.kind === 'ending' ? 'rgba(34,211,238,0.28)' : 'rgba(255,255,255,0.12)')"
                          stroke="rgba(255,255,255,0.22)"
                          :stroke-width="(hoveredId === n.id || selectedId === n.id) ? 3 : 2"
                        />
                        <circle
                          :r="(n.kind === 'ending' ? 14 : 12) + ((hoveredId === n.id || selectedId === n.id) ? 2 : 0)"
                          :fill="n.kind === 'ending' ? 'rgba(34,211,238,0.85)' : 'rgba(217,70,239,0.75)'"
                          opacity="0.85"
                        />
                        <text x="0" y="44" text-anchor="middle" font-size="12" fill="rgba(255,255,255,0.78)" font-family="ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace">
                          {{ n.label.length > 14 ? n.label.slice(0, 14) + '…' : n.label }}
                        </text>
                      </g>
                    </g>
                  </g>
                </svg>

                <div v-if="selectedNodeInfo" class="absolute right-3 top-3 w-[320px] max-w-[92%] rounded-xl border border-white/10 bg-black/55 backdrop-blur-md p-4">
                  <div class="flex items-center justify-between gap-2">
                    <div class="text-xs tracking-[0.22em] uppercase text-white/50 font-semibold">{{ selectedNodeInfo.kind === 'ending' ? 'Ending' : 'Node' }}</div>
                    <button class="text-xs text-white/60 hover:text-white" @click="selectedId = ''">关闭</button>
                  </div>

                  <div class="mt-2 font-mono text-white/90 break-all">{{ selectedNodeInfo.id }}</div>

                  <template v-if="selectedNodeInfo.kind === 'ending'">
                    <div class="mt-3 text-sm text-white/80 leading-relaxed">{{ selectedNodeInfo.description }}</div>
                    <div class="mt-3 text-xs text-white/55">type: {{ selectedNodeInfo.type }}</div>
                  </template>
                  <template v-else>
                    <div v-if="selectedNodeInfo.endingKey" class="mt-3 text-xs text-white/55">endingKey: {{ selectedNodeInfo.endingKey }}</div>
                    <div v-if="selectedNodeInfo.characters?.length" class="mt-3 text-xs text-white/55">characters: {{ selectedNodeInfo.characters.join(' / ') }}</div>
                    <div class="mt-3 text-sm text-white/80 leading-relaxed max-h-[140px] overflow-auto">{{ selectedNodeInfo.content }}</div>
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
  </div>
</template>
