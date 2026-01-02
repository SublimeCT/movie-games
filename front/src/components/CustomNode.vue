<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core';
import { computed } from 'vue';

const props = defineProps<{
  id: string;
  data: {
    label: string;
    kind: 'story' | 'ending';
    highlighted?: boolean;
  };
}>();

const isEnding = computed(() => props.data.kind === 'ending');
const isHighlighted = computed(() => props.data.highlighted);
</script>

<template>
  <div
    class="custom-node w-full h-full rounded-xl border backdrop-blur-md flex flex-col justify-center px-4 py-2 transition-all duration-300 cursor-pointer shadow-lg group hover:scale-105"
    :class="[
      isEnding
        ? 'bg-cyan-900/20 border-cyan-500/30 hover:border-cyan-400'
        : 'bg-neutral-900/40 border-purple-500/20 hover:border-purple-400',
      isHighlighted ? 'ring-2 ring-purple-500/50 shadow-[0_0_15px_rgba(168,85,247,0.3)]' : ''
    ]"
  >
    <Handle type="target" :position="Position.Left" class="!bg-transparent !border-0" />
    
    <div class="flex items-center justify-between mb-1 pointer-events-none">
      <span :class="['text-[10px] font-mono uppercase tracking-wider', isEnding ? 'text-cyan-400' : 'text-purple-400']">
        {{ isEnding ? 'ENDING' : 'NODE' }}
      </span>
      <div v-if="isHighlighted" class="w-1.5 h-1.5 rounded-full bg-green-400 animate-pulse shadow-[0_0_5px_rgba(74,222,128,0.8)]"></div>
    </div>
    <div class="text-xs font-bold text-white/90 truncate font-mono pointer-events-none">{{ data.label }}</div>
    <div class="absolute inset-0 rounded-xl bg-gradient-to-r from-white/0 via-white/5 to-white/0 opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none"></div>

    <Handle type="source" :position="Position.Right" class="!bg-transparent !border-0" />
  </div>
</template>

<style scoped>
:deep(.vue-flow__handle) {
  width: 1px;
  height: 1px;
  opacity: 0;
}
</style>
