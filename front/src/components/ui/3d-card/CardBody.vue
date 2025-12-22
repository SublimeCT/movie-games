<script setup lang="ts">
import { computed, inject, type Ref } from 'vue';

// biome-ignore lint/style/noNonNullAssertion: Injected values are guaranteed to be provided
const elementX = inject<Ref<number>>('elementX')!;
// biome-ignore lint/style/noNonNullAssertion: Injected values are guaranteed to be provided
const elementY = inject<Ref<number>>('elementY')!;
// biome-ignore lint/style/noNonNullAssertion: Injected values are guaranteed to be provided
const elementWidth = inject<Ref<number>>('elementWidth')!;
// biome-ignore lint/style/noNonNullAssertion: Injected values are guaranteed to be provided
const elementHeight = inject<Ref<number>>('elementHeight')!;
// biome-ignore lint/style/noNonNullAssertion: Injected values are guaranteed to be provided
const isOutside = inject<Ref<boolean>>('isOutside')!;

const transform = computed(() => {
  if (isOutside.value) {
    return 'rotateY(0deg) rotateX(0deg) translateZ(0px)';
  }

  const x = (elementX.value - elementWidth.value / 2) / 25;
  const y = (elementY.value - elementHeight.value / 2) / 25;

  return `rotateY(${x}deg) rotateX(${-y}deg) translateZ(0px)`;
});
</script>

<template>
  <div
    class="relative h-96 w-96 [transform-style:preserve-3d] transition-all duration-200 ease-linear"
    :style="{ transform }"
  >
    <slot />
  </div>
</template>
