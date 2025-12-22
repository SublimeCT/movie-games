
<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  min: number;
  max: number;
  step?: number;
  modelValue: [number, number];
}>();

const emit =
  defineEmits<(e: 'update:modelValue', value: [number, number]) => void>();

const minVal = computed({
  get: () => props.modelValue[0],
  set: (val) => {
    const newVal: [number, number] = [
      Math.min(val, props.modelValue[1]),
      props.modelValue[1],
    ];
    emit('update:modelValue', newVal);
  },
});

const maxVal = computed({
  get: () => props.modelValue[1],
  set: (val) => {
    const newVal: [number, number] = [
      props.modelValue[0],
      Math.max(val, props.modelValue[0]),
    ];
    emit('update:modelValue', newVal);
  },
});

// Calculate percentages for track fill
const minPercent = computed(
  () => ((minVal.value - props.min) / (props.max - props.min)) * 100,
);
const maxPercent = computed(
  () => ((maxVal.value - props.min) / (props.max - props.min)) * 100,
);
</script>

<template>
  <div class="relative w-full h-6 flex items-center select-none touch-none group">
    <!-- Track Background -->
    <div class="absolute w-full h-2 bg-neutral-800 rounded-full overflow-hidden">
        <!-- Active Range -->
        <div 
            class="absolute h-full bg-purple-600 transition-all duration-75"
            :style="{ left: `${minPercent}%`, width: `${maxPercent - minPercent}%` }"
        ></div>
    </div>

    <!-- Thumb Min -->
    <input 
        type="range" 
        :min="min" 
        :max="max" 
        :step="step || 1"
        v-model.number="minVal"
        class="absolute w-full h-2 appearance-none bg-transparent pointer-events-none [&::-webkit-slider-thumb]:pointer-events-auto [&::-webkit-slider-thumb]:w-5 [&::-webkit-slider-thumb]:h-5 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:shadow-lg [&::-webkit-slider-thumb]:cursor-grab [&::-webkit-slider-thumb]:transition-transform [&::-webkit-slider-thumb]:hover:scale-110 z-20"
    />

    <!-- Thumb Max -->
    <input 
        type="range" 
        :min="min" 
        :max="max" 
        :step="step || 1"
        v-model.number="maxVal"
        class="absolute w-full h-2 appearance-none bg-transparent pointer-events-none [&::-webkit-slider-thumb]:pointer-events-auto [&::-webkit-slider-thumb]:w-5 [&::-webkit-slider-thumb]:h-5 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:shadow-lg [&::-webkit-slider-thumb]:cursor-grab [&::-webkit-slider-thumb]:transition-transform [&::-webkit-slider-thumb]:hover:scale-110 z-20"
    />
  </div>
</template>
