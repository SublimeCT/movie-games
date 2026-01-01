<script setup lang="ts">
import { createNoise3D } from 'simplex-noise';
import { onMounted, onUnmounted, ref } from 'vue';

interface Props {
  particleCount?: number;
  rangeY?: number;
  baseHue?: number;
  baseSpeed?: number;
  rangeSpeed?: number;
  baseRadius?: number;
  rangeRadius?: number;
  backgroundColor?: string;
  containerClass?: string;
}

const props = withDefaults(defineProps<Props>(), {
  particleCount: 1700,
  rangeY: 400,
  baseHue: 220,
  baseSpeed: 2.0,
  rangeSpeed: 2,
  baseRadius: 1,
  rangeRadius: 2,
  backgroundColor: '#000000',
  containerClass: '',
});

const canvasRef = ref<HTMLCanvasElement | null>(null);
const containerRef = ref<HTMLDivElement | null>(null);
const noise3D = createNoise3D();
let animationFrameId: number;
let particles: Particle[] = [];
let tick = 0;

// Particle state stored in localStorage
const STORAGE_KEY = 'mg_vortex_particles';
const TIME_KEY = 'mg_vortex_time';

const getStoredTime = (): number => {
  try {
    const stored = localStorage.getItem(TIME_KEY);
    return stored ? Number.parseFloat(stored) : 0;
  } catch {
    return 0;
  }
};

const saveTime = (time: number) => {
  localStorage.setItem(TIME_KEY, time.toString());
};

const clearTime = () => {
  localStorage.removeItem(TIME_KEY);
  localStorage.removeItem(STORAGE_KEY);
};

tick = getStoredTime();

class Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  ttl: number;
  speed: number;
  radius: number;
  hue: number;
  angle: number;
  distance: number;

  constructor(canvasWidth: number, canvasHeight: number) {
    // Spawn particles across the ENTIRE canvas, not just view area
    // This ensures particles are visible when canvas is larger than viewport
    this.x = Math.random() * canvasWidth;
    this.y = Math.random() * canvasHeight;
    this.vx = 0;
    this.vy = 0;
    this.ttl = Math.random() * 200 + 300;
    this.life = Math.random() * this.ttl;
    this.speed = props.baseSpeed + Math.random() * props.rangeSpeed;
    this.radius = props.baseRadius + Math.random() * props.rangeRadius;
    this.hue = props.baseHue + Math.random() * 30;

    // Calculate distance from canvas center
    const dx = this.x - canvasWidth / 2;
    const dy = this.y - canvasHeight / 2;
    this.distance = Math.sqrt(dx * dx + dy * dy);
    this.angle = Math.atan2(dy, dx);
  }

  update(canvasWidth: number, canvasHeight: number, time: number) {
    const centerX = canvasWidth / 2;
    const centerY = canvasHeight / 2;

    // Spiral movement
    this.angle += 0.005 * this.speed;
    this.distance -= 0.5 * this.speed;

    // Add some noise
    const noise = noise3D(this.x / 200, this.y / 200, time * 0.0002) * 20;

    this.x = centerX + Math.cos(this.angle) * (this.distance + noise);
    this.y = centerY + Math.sin(this.angle) * (this.distance + noise);

    this.life++;

    // Reset if too close to center or dead
    if (this.distance < 2) {
      this.reset(canvasWidth, canvasHeight);
    }
  }

  reset(canvasWidth: number, canvasHeight: number) {
    const centerX = canvasWidth / 2;
    const centerY = canvasHeight / 2;
    // Spawn at edges - use CANVAS size for maximum coverage
    const maxDim = Math.max(canvasWidth, canvasHeight);
    const angle = Math.random() * Math.PI * 2;
    const distance = maxDim * 0.6 + Math.random() * 100;

    this.angle = angle;
    this.distance = distance;
    this.x = centerX + Math.cos(angle) * distance;
    this.y = centerY + Math.sin(angle) * distance;

    this.life = 0;
    this.ttl = 10000;
    this.hue = props.baseHue + Math.random() * 30;
  }

  draw(ctx: CanvasRenderingContext2D, viewWidth: number, viewHeight: number) {
    ctx.beginPath();
    ctx.arc(this.x, this.y, this.radius, 0, Math.PI * 2);

    // Calculate fade radius based on view dimensions (not canvas)
    const minDim = Math.min(viewWidth, viewHeight);
    const fadeRadius = minDim * 0.1;

    // Opacity fades out as it approaches the center
    const opacity = Math.min(1, this.distance / fadeRadius);

    ctx.fillStyle = `hsla(${this.hue}, 60%, 50%, ${0.8 * opacity})`;
    ctx.fill();
    ctx.closePath();
  }
}

// Get canvas size - use diagonal * 2 to ensure full coverage during rotation
const getCanvasSize = (
  viewWidth: number,
  viewHeight: number,
): { width: number; height: number } => {
  const diagonal = Math.sqrt(viewWidth ** 2 + viewHeight ** 2);
  const size = Math.ceil(diagonal * 2);
  return { width: size, height: size };
};

const initParticles = (canvasWidth: number, canvasHeight: number) => {
  particles = [];
  for (let i = 0; i < props.particleCount; i++) {
    particles.push(new Particle(canvasWidth, canvasHeight));
  }
};

const render = () => {
  if (!canvasRef.value || !containerRef.value) return;
  const ctx = canvasRef.value.getContext('2d');
  if (!ctx) return;

  const canvasWidth = canvasRef.value.width;
  const canvasHeight = canvasRef.value.height;
  const viewWidth = containerRef.value.clientWidth;
  const viewHeight = containerRef.value.clientHeight;

  // Trail effect
  ctx.fillStyle = 'rgba(0, 0, 0, 0.1)';
  ctx.fillRect(0, 0, canvasWidth, canvasHeight);

  tick++;

  // Throttled save for performance
  if (tick % 5 === 0) {
    saveTime(tick);
  }

  particles.forEach((p) => {
    p.update(canvasWidth, canvasHeight, tick);
    p.draw(ctx, viewWidth, viewHeight);
  });

  animationFrameId = requestAnimationFrame(render);
};

const handleResize = () => {
  if (!containerRef.value || !canvasRef.value) return;

  const viewWidth = containerRef.value.clientWidth;
  const viewHeight = containerRef.value.clientHeight;

  // Use larger canvas size for rotation coverage
  const { width, height } = getCanvasSize(viewWidth, viewHeight);
  canvasRef.value.width = width;
  canvasRef.value.height = height;

  // Center the canvas using CSS
  canvasRef.value.style.position = 'absolute';
  canvasRef.value.style.left = '50%';
  canvasRef.value.style.top = '50%';
  canvasRef.value.style.transform = 'translate(-50%, -50%)';

  initParticles(width, height);
};

onMounted(() => {
  handleResize();
  window.addEventListener('resize', handleResize);
  render();
});

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
  cancelAnimationFrame(animationFrameId);
  clearTime();
});
</script>

<template>
  <div ref="containerRef" :class="['relative h-full w-full overflow-hidden', containerClass]">
    <canvas ref="canvasRef" class="absolute z-0"></canvas>
    <!-- Content centered on top -->
    <div class="relative z-10 h-full w-full flex flex-col items-center justify-center">
      <slot />
    </div>
  </div>
</template>
