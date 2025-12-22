<script setup lang="ts">
import { inject, type Ref, ref, watchEffect } from 'vue';

const props = defineProps({
  translateZ: {
    type: [Number, String],
    default: 0,
  },
  as: {
    type: String,
    default: 'div',
  },
});

// biome-ignore lint/style/noNonNullAssertion: Injected values are guaranteed to be provided
const mouseEnter = inject<Ref<boolean>>('mouseEnter')!;
const transform = ref('translateZ(0px)');

watchEffect(() => {
  if (mouseEnter.value) {
    transform.value = `translateZ(${props.translateZ}px)`;
  } else {
    transform.value = 'translateZ(0px)';
  }
});
</script>

<template>
  <component
    :is="as"
    class="w-full transition-all duration-200 ease-linear [transform-style:preserve-3d]"
    :style="{ transform }"
  >
    <slot />
  </component>
</template>
