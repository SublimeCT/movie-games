<script setup lang="ts">
import { useMouseInElement } from '@vueuse/core';
import { computed, ref } from 'vue';

/**
 * A 3D Card component that rotates based on mouse position.
 * Creates a depth effect using CSS transforms.
 * Inspired by Aceternity UI / Inspira UI.
 */

const target = ref<HTMLElement | null>(null);
const { elementX, elementY, elementWidth, elementHeight, isOutside } =
  useMouseInElement(target);

const transform = computed(() => {
  if (isOutside.value) {
    return 'rotateY(0deg) rotateX(0deg) translateZ(0px)';
  }

  const MAX_ROTATION = 20;

  const x = (elementX.value - elementWidth.value / 2) / elementWidth.value;
  const y = (elementY.value - elementHeight.value / 2) / elementHeight.value;

  // Rotate Y is based on X position (left/right)
  // Rotate X is based on Y position (up/down) - inverted
  return `rotateY(${x * MAX_ROTATION}deg) rotateX(${-y * MAX_ROTATION}deg) translateZ(0px)`;
});
</script>

<template>
  <div 
    ref="target"
    class="flex items-center justify-center"
    style="perspective: 1000px;"
  >
    <div 
      class="transition-transform duration-200 ease-linear w-full h-full"
      :style="{ transform, transformStyle: 'preserve-3d' }"
    >
      <slot />
    </div>
  </div>
</template>
