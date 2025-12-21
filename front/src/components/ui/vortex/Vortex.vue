<script setup lang="ts">
 import { onMounted, onUnmounted, ref } from "vue";
import { createNoise3D } from "simplex-noise";

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
  particleCount: 700,
  rangeY: 400,
  baseHue: 220,
  baseSpeed: 2.0,
  rangeSpeed: 1.5,
  baseRadius: 1,
  rangeRadius: 2,
  backgroundColor: "#000000",
  containerClass: "",
});

const canvasRef = ref<HTMLCanvasElement | null>(null);
const containerRef = ref<HTMLDivElement | null>(null);
const noise3D = createNoise3D();
let animationFrameId: number;
let particles: Particle[] = [];
let tick = 0;

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

  constructor(width: number, height: number) {
    this.x = Math.random() * width;
    this.y = Math.random() * height;
    this.vx = 0;
    this.vy = 0;
    this.ttl = Math.random() * 200 + 300;
    this.life = Math.random() * this.ttl; // Randomize initial life to prevent synchronized fading
    this.speed = props.baseSpeed + Math.random() * props.rangeSpeed;
    this.radius = props.baseRadius + Math.random() * props.rangeRadius;
    this.hue = props.baseHue + Math.random() * 30;
    
    // Vortex specific initialization
    const dx = this.x - width / 2;
    const dy = this.y - height / 2;
    this.distance = Math.sqrt(dx * dx + dy * dy);
    this.angle = Math.atan2(dy, dx);
  }

  update(width: number, height: number, time: number) {
    const centerX = width / 2;
    const centerY = height / 2;
    
    // Spiral movement
    this.angle += 0.005 * this.speed;
    this.distance -= 0.5 * this.speed;
    
    // Add some noise
    const noise = noise3D(this.x / 200, this.y / 200, time * 0.0002) * 20;
    
    this.x = centerX + Math.cos(this.angle) * (this.distance + noise);
    this.y = centerY + Math.sin(this.angle) * (this.distance + noise);

    this.life++;

    // Reset if too close to center or dead
    if (this.distance < 10 || this.life > this.ttl) {
      this.reset(width, height);
    }
  }

  reset(width: number, height: number) {
    const centerX = width / 2;
    const centerY = height / 2;
    // Spawn at edges or random
    const angle = Math.random() * Math.PI * 2;
    const distance = Math.max(width, height) * 0.5 + Math.random() * 100; // Spawn closer
    
    this.angle = angle;
    this.distance = distance;
    this.x = centerX + Math.cos(angle) * distance;
    this.y = centerY + Math.sin(angle) * distance;
    
    this.life = 0;
    this.ttl = Math.random() * 200 + 300; // Consistent long TTL
    this.hue = props.baseHue + Math.random() * 30;
  }

  draw(ctx: CanvasRenderingContext2D) {
    ctx.beginPath();
    ctx.arc(this.x, this.y, this.radius, 0, Math.PI * 2);
    ctx.fillStyle = `hsla(${this.hue}, 60%, 50%, ${0.8 * (1 - this.life / this.ttl)})`;
    ctx.fill();
    ctx.closePath();
  }
}

const initParticles = (width: number, height: number) => {
  particles = [];
  for (let i = 0; i < props.particleCount; i++) {
    particles.push(new Particle(width, height));
  }
};

const render = () => {
  if (!canvasRef.value) return;
  const ctx = canvasRef.value.getContext("2d");
  if (!ctx) return;

  const width = canvasRef.value.width;
  const height = canvasRef.value.height;

  // Trail effect
  ctx.fillStyle = `rgba(0, 0, 0, 0.1)`; 
  ctx.fillRect(0, 0, width, height);

  tick++;

  particles.forEach((p) => {
    p.update(width, height, tick);
    p.draw(ctx);
  });

  animationFrameId = requestAnimationFrame(render);
};

const handleResize = () => {
  if (containerRef.value && canvasRef.value) {
    canvasRef.value.width = containerRef.value.clientWidth;
    canvasRef.value.height = containerRef.value.clientHeight;
    initParticles(canvasRef.value.width, canvasRef.value.height);
  }
};

onMounted(() => {
  handleResize();
  window.addEventListener("resize", handleResize);
  render();
});

onUnmounted(() => {
  window.removeEventListener("resize", handleResize);
  cancelAnimationFrame(animationFrameId);
});
</script>

<template>
  <div ref="containerRef" :class="['relative h-full w-full overflow-hidden', containerClass]">
    <canvas ref="canvasRef" class="absolute inset-0 h-full w-full"></canvas>
    <!-- Added flex centering to the content wrapper -->
    <div class="relative z-10 h-full w-full flex flex-col items-center justify-center">
      <slot />
    </div>
  </div>
</template>
