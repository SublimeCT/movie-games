<script setup lang="ts">
import { type InjectionKey, inject, type Ref } from 'vue';

// --- Types & Context ---
type MouseEnterContext = Ref<boolean>;
const MouseEnterKey = Symbol('MouseEnter') as InjectionKey<MouseEnterContext>;

export const useMouseEnter = () => {
  const context = inject(MouseEnterKey);
  if (!context) {
    throw new Error('useMouseEnter must be used within a CardContainer');
  }
  return context;
};

// --- Components (Inline for simplicity, or split if preferred) ---
// Since we are in a single file for the "tool", I will export them as named exports if possible,
// but Vue SFCs export default. So I will create a "CardContainer.vue", "CardBody.vue", "CardItem.vue"
// pattern if I can.
// For now, I'll create a single file that exports the Container as default and others... wait.
// Vue SFC only allows one default export.
// I will create separate files.
</script>

<template>
  <!-- Placeholder -->
  <div></div>
</template>
