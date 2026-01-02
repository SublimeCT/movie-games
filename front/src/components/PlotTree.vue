<script setup lang="ts">
import { Background } from '@vue-flow/background';
import { Controls } from '@vue-flow/controls';
import { VueFlow } from '@vue-flow/core';
import { MiniMap } from '@vue-flow/minimap';
import { computed, nextTick, watch } from 'vue';
import type { Ending, StoryNode } from '../types/movie';
import CustomNode from './CustomNode.vue';
import '@vue-flow/core/dist/style.css';
import '@vue-flow/core/dist/theme-default.css';
import '@vue-flow/controls/dist/style.css';
import '@vue-flow/minimap/dist/style.css';

const props = defineProps<{
  nodes: Record<string, StoryNode>;
  endings: Record<string, Ending>;
  startNodeId: string;
  preventScrolling?: boolean;
  highlightedIds?: Set<string>;
}>();

const emit = defineEmits<{
  (e: 'nodeClick', id: string): void;
  (e: 'paneClick'): void;
}>();

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

const treeGraph = computed(() => {
  const nodes = props.nodes;
  const endings = props.endings;
  const root = props.startNodeId;

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
      if (seenTargets.has(to)) continue;
      seenTargets.add(to);

      if (nodes[to]) list.push({ to, label: c.text });
      else if (knownEndingKeys.has(to)) list.push({ to, label: c.text });
      else if (to === 'END') list.push({ to, label: c.text });
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

  // Handle fallback start node logic if needed (though startNodeId prop should handle it)
  // We'll trust the prop but ensure 'start' is included if it exists and wasn't reached
  if (nodes.start && !visited.has('start') && root !== 'start') {
     // This logic was in the original components, but if startNodeId is correctly passed, we might not need it.
     // Keeping it simple: rely on BFS from startNodeId.
  }

  const byDepth = new Map<number, string[]>();
  for (const id of visited) {
    const d = depth.get(id) ?? 0;
    if (!byDepth.has(d)) byDepth.set(d, []);
    byDepth.get(d)?.push(id);
  }

  const maxDepth = Math.max(0, ...Array.from(byDepth.keys()));

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

const vueFlowNodes = computed(() => {
  return treeGraph.value.nodes.map((n) => ({
    id: n.id,
    position: { x: n.x, y: n.y },
    data: {
      label: n.label,
      kind: n.kind,
      highlighted: props.highlightedIds?.has(n.id) ?? false,
    },
    type: 'custom',
    style: { width: `${n.w}px`, height: `${n.h}px` },
    draggable: false,
    connectable: false,
  }));
});

const vueFlowEdges = computed(() => {
  return treeGraph.value.edges.map((e) => {
    const isHighlighted = (props.highlightedIds?.has(e.from) && props.highlightedIds?.has(e.to)) ?? false;
    return {
      id: `${e.from}-${e.to}`,
      source: e.from,
      target: e.to,
      animated: isHighlighted,
      style: isHighlighted
        ? { stroke: '#d946ef', strokeWidth: 3 }
        : { stroke: '#9333ea', strokeOpacity: 0.3 },
      markerEnd: 'arrowclosed',
    };
  });
});

let vueFlowInstance: any = null;
const onVueFlowInit = (instance: any) => {
  vueFlowInstance = instance;
  instance.fitView();
};

const onVueFlowNodeClick = (event: { node: { id: string } }) => {
  emit('nodeClick', event.node.id);
};

const onPaneClick = () => {
  emit('paneClick');
};

// Expose fitView to parent
defineExpose({
  fitView: () => {
    nextTick(() => {
      vueFlowInstance?.fitView();
    });
  },
});

// Watch for data changes to auto-fit
watch(
  () => [props.nodes, props.endings],
  () => {
    nextTick(() => {
      vueFlowInstance?.fitView();
    });
  },
  { deep: true }
);
</script>

<template>
  <div class="w-full h-full relative">
    <VueFlow
      :nodes="vueFlowNodes"
      :edges="vueFlowEdges"
      :default-viewport="{ zoom: 0.9 }"
      :nodes-draggable="false"
      :nodes-connectable="false"
      :elements-selectable="false"
      :pan-on-drag="true"
      :zoom-on-scroll="true"
      :zoom-on-pinch="true"
      :zoom-on-double-click="true"
      :prevent-scrolling="props.preventScrolling ?? true"
      @node-click="onVueFlowNodeClick"
      @pane-click="onPaneClick"
      @init="onVueFlowInit"
      fit-view-on-init
      class="h-full w-full"
    >
      <Background />
      <Controls />
      <MiniMap />
      <template #node-custom="nodeProps">
        <CustomNode v-bind="nodeProps" />
      </template>
    </VueFlow>
  </div>
</template>

<style>
/* Global styles for VueFlow customization if needed */
.vue-flow__minimap {
  background-color: rgba(0, 0, 0, 0.8) !important;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 0.5rem;
}

.vue-flow__minimap-node {
  fill: #333 !important;
  stroke: #555 !important;
}

.vue-flow__minimap-node.selected {
  fill: #9333ea !important;
  stroke: #d8b4fe !important;
}

.vue-flow__controls {
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 0.5rem;
  overflow: hidden;
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
}

.vue-flow__controls-button {
  background-color: #262626 !important;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1) !important;
  fill: #e5e5e5 !important;
  border-radius: 0 !important;
  transition: background-color 0.2s;
}

.vue-flow__controls-button:last-child {
  border-bottom: none !important;
}

.vue-flow__controls-button:hover {
  background-color: #404040 !important;
}

.vue-flow__controls-button svg {
  fill: currentColor !important;
}

/* Ensure node content is interactive */
.custom-node {
  pointer-events: all;
}
</style>
