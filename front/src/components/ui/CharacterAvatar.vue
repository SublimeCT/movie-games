<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  name?: string;
  gender?: string; // 'Male', 'Female', 'Other'
  emotion?: string; // 'happy', 'sad', 'angry', 'surprised', 'neutral'
  avatarPath?: string;
  className?: string;
}>();

// Simple hash function for consistent randomness
const hash = (str: string) => {
  let h = 0;
  for (let i = 0; i < str.length; i++) {
    h = (Math.imul(31, h) + str.charCodeAt(i)) | 0;
  }
  return h;
};

const seed = computed(() => hash(props.name || 'default'));

// Colors
const skinTone = computed(() => {
  const tones = ['#f5d0b0', '#eac096', '#e0ac69', '#8d5524', '#c68642'];
  return tones[Math.abs(seed.value) % tones.length];
});

const hairColor = computed(() => {
  const colors = [
    '#090806',
    '#2c222b',
    '#71635a',
    '#b7a69e',
    '#d6c4c2',
    '#b55239',
  ];
  return colors[Math.abs(seed.value) % colors.length];
});

// Features based on gender
const isFemale = computed(() => props.gender?.toLowerCase() === 'female');

// Mouth path based on emotion
const mouthPath = computed(() => {
  const e = props.emotion?.toLowerCase() || 'neutral';
  if (e.includes('happy') || e.includes('joy')) return 'M 35 75 Q 50 85 65 75'; // Smile
  if (e.includes('sad') || e.includes('grief')) return 'M 35 80 Q 50 70 65 80'; // Frown
  if (e.includes('angry')) return 'M 40 80 L 60 80'; // Straight line
  if (e.includes('surprised') || e.includes('shock'))
    return 'M 45 75 A 5 5 0 1 0 55 75 A 5 5 0 1 0 45 75'; // Open O
  return 'M 38 78 Q 50 82 62 78'; // Neutral
});

// Eyebrow path
const eyebrowPath = computed(() => {
  const e = props.emotion?.toLowerCase() || 'neutral';
  if (e.includes('angry')) return ['M 25 45 L 40 50', 'M 75 45 L 60 50'];
  if (e.includes('sad')) return ['M 25 45 L 40 40', 'M 75 45 L 60 40'];
  if (e.includes('surprised'))
    return ['M 25 35 Q 32 30 40 35', 'M 60 35 Q 68 30 75 35'];
  return ['M 25 42 Q 32 40 40 42', 'M 60 42 Q 68 40 75 42'];
});

const imageSrc = computed(() => {
  const v = (props.avatarPath || '').trim();
  return v ? v : undefined;
});
</script>

<template>
  <div :class="['relative w-24 h-24 transition-transform hover:scale-105 duration-300', className]">
    <img
      v-if="imageSrc"
      :src="imageSrc"
      :alt="name || 'avatar'"
      class="w-full h-full object-cover rounded-full drop-shadow-lg"
      draggable="false"
    />
    <svg v-else viewBox="0 0 100 100" class="w-full h-full drop-shadow-lg filter">
      <!-- Defs for gradients/filters -->
      <defs>
        <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
          <feGaussianBlur stdDeviation="2" result="blur" />
          <feComposite in="SourceGraphic" in2="blur" operator="over" />
        </filter>
      </defs>

      <!-- Hair Back (Long hair) -->
      <path v-if="isFemale" d="M 15 30 Q 5 60 10 90 L 90 90 Q 95 60 85 30 Z" :fill="hairColor" />

      <!-- Face Shape -->
      <path 
        :d="isFemale ? 'M 20 30 Q 20 90 50 95 Q 80 90 80 30 Q 80 10 50 10 Q 20 10 20 30' : 'M 20 20 L 20 80 Q 50 95 80 80 L 80 20 Q 50 5 20 20'" 
        :fill="skinTone" 
      />

      <!-- Hair Front/Bangs -->
      <path v-if="isFemale" d="M 20 30 Q 50 10 80 30 Q 80 10 50 5 Q 20 10 20 30" :fill="hairColor" />
      <path v-else d="M 20 20 Q 50 5 80 20 L 80 25 Q 50 15 20 25 Z" :fill="hairColor" />

      <!-- Eyes -->
      <circle cx="35" cy="55" r="4" fill="#1a1a1a" />
      <circle cx="65" cy="55" r="4" fill="#1a1a1a" />

      <!-- Eyebrows -->
      <path :d="eyebrowPath[0]" stroke="#1a1a1a" stroke-width="2" fill="none" />
      <path :d="eyebrowPath[1]" stroke="#1a1a1a" stroke-width="2" fill="none" />

      <!-- Mouth -->
      <path :d="mouthPath" stroke="#3a2a1a" stroke-width="3" fill="none" stroke-linecap="round" />

      <!-- Blush (Optional) -->
      <circle v-if="emotion?.includes('happy') || emotion?.includes('shy')" cx="25" cy="65" r="5" fill="#ff0000" opacity="0.1" />
      <circle v-if="emotion?.includes('happy') || emotion?.includes('shy')" cx="75" cy="65" r="5" fill="#ff0000" opacity="0.1" />

    </svg>
    
    <!-- Name Tag (Optional Overlay) -->
    <!-- <div class="absolute -bottom-2 left-1/2 -translate-x-1/2 bg-black/70 text-white text-[10px] px-2 py-0.5 rounded-full whitespace-nowrap opacity-0 group-hover:opacity-100 transition-opacity">
        {{ name }}
    </div> -->
  </div>
</template>
