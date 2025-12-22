<script setup lang="ts">
import { useMouseInElement } from '@vueuse/core';
import { provide, ref } from 'vue';

const containerRef = ref<HTMLElement | null>(null);

const { elementX, elementY, elementWidth, elementHeight, isOutside } =
  useMouseInElement(containerRef);

const mouseEnter = ref(false);

provide('mouseEnter', mouseEnter);
provide('elementX', elementX);
provide('elementY', elementY);
provide('elementWidth', elementWidth);
provide('elementHeight', elementHeight);
provide('isOutside', isOutside);

const handleMouseEnter = () => {
  mouseEnter.value = true;
};

const handleMouseLeave = () => {
  mouseEnter.value = false;
};
</script>

<template>
  <div
    ref="containerRef"
    class="flex items-center justify-center p-2 py-20"
    style="perspective: 1000px;"
    @mouseenter="handleMouseEnter"
    @mouseleave="handleMouseLeave"
  >
    <slot />
  </div>
</template>
