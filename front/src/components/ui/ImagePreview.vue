<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';

interface Props {
  src: string;
  open: boolean;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  (e: 'close'): void;
}>();

const containerRef = ref<HTMLElement | null>(null);
const imageRef = ref<HTMLImageElement | null>(null);

// 缩放和平移状态
const scale = ref(1);
const translateX = ref(0);
const translateY = ref(0);

// 拖拽状态
const isDragging = ref(false);
const startX = ref(0);
const startY = ref(0);

// 双指缩放状态
const initialPinchDistance = ref(0);
const initialScale = ref(0);

// 双击检测状态
const lastTapTime = ref(0);
const tapTimeout = ref<ReturnType<typeof setTimeout> | null>(null);

// 重置视图
const resetView = () => {
  scale.value = 1;
  translateX.value = 0;
  translateY.value = 0;
};

// 放大视图
const zoomIn = () => {
  scale.value = Math.min(5, scale.value * 2);
  translateX.value = 0;
  translateY.value = 0;
};

// 关闭预览
const handleClose = () => {
  emit('close');
};

// 计算两指距离
const getPinchDistance = (touch1: Touch, touch2: Touch) => {
  const dx = touch1.clientX - touch2.clientX;
  const dy = touch1.clientY - touch2.clientY;
  return Math.sqrt(dx * dx + dy * dy);
};

// 计算两指中心点
const getPinchCenter = (touch1: Touch, touch2: Touch) => {
  return {
    x: (touch1.clientX + touch2.clientX) / 2,
    y: (touch1.clientY + touch2.clientY) / 2,
  };
};

// 鼠标滚轮缩放
const handleWheel = (e: WheelEvent) => {
  e.preventDefault();
  const delta = e.deltaY > 0 ? -0.1 : 0.1;
  const newScale = Math.max(0.5, Math.min(5, scale.value + delta));
  scale.value = newScale;
};

// 拖拽开始
const handleMouseDown = (e: MouseEvent) => {
  if (e.button !== 0) return; // 只响应左键
  isDragging.value = true;
  startX.value = e.clientX - translateX.value;
  startY.value = e.clientY - translateY.value;
};

// 拖拽移动
const handleMouseMove = (e: MouseEvent) => {
  if (!isDragging.value) return;
  translateX.value = e.clientX - startX.value;
  translateY.value = e.clientY - startY.value;
};

// 拖拽结束
const handleMouseUp = () => {
  isDragging.value = false;
};

// 触摸开始
const handleTouchStart = (e: TouchEvent) => {
  if (e.touches.length === 1) {
    // 单指：准备拖动
    isDragging.value = true;
    startX.value = e.touches[0].clientX - translateX.value;
    startY.value = e.touches[0].clientY - translateY.value;
  } else if (e.touches.length === 2) {
    // 双指：准备缩放
    isDragging.value = false;
    initialPinchDistance.value = getPinchDistance(e.touches[0], e.touches[1]);
    initialScale.value = scale.value;
  }
};

// 触摸移动
const handleTouchMove = (e: TouchEvent) => {
  e.preventDefault(); // 防止页面滚动

  if (e.touches.length === 1 && isDragging.value) {
    // 单指拖动
    translateX.value = e.touches[0].clientX - startX.value;
    translateY.value = e.touches[0].clientY - startY.value;
  } else if (e.touches.length === 2) {
    // 双指缩放
    const currentDistance = getPinchDistance(e.touches[0], e.touches[1]);
    const scaleRatio = currentDistance / initialPinchDistance.value;
    const newScale = Math.max(0.5, Math.min(5, initialScale.value * scaleRatio));
    scale.value = newScale;
  }
};

// 触摸结束
const handleTouchEnd = (e: TouchEvent) => {
  isDragging.value = false;

  // 检测双击
  const currentTime = Date.now();
  const timeDiff = currentTime - lastTapTime.value;

  if (timeDiff < 300 && timeDiff > 0) {
    // 双击：根据当前缩放状态决定放大或重置
    if (scale.value === 1) {
      zoomIn();
    } else {
      resetView();
    }
    lastTapTime.value = 0;
  } else {
    // 单击：记录时间，延迟检测
    lastTapTime.value = currentTime;
  }
};

// 双击重置
const handleDoubleClick = () => {
  resetView();
};

// ESC 键关闭
const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && props.open) {
    handleClose();
  }
};

// 监听全局事件
onMounted(() => {
  document.addEventListener('mousemove', handleMouseMove);
  document.addEventListener('mouseup', handleMouseUp);
  document.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  document.removeEventListener('mousemove', handleMouseMove);
  document.removeEventListener('mouseup', handleMouseUp);
  document.removeEventListener('keydown', handleKeydown);
});

// 监听触摸事件（只在预览打开时）
watch(() => props.open, (isOpen) => {
  if (isOpen) {
    document.addEventListener('touchmove', handleTouchMove, { passive: false });
    document.addEventListener('touchend', handleTouchEnd);
  } else {
    document.removeEventListener('touchmove', handleTouchMove);
    document.removeEventListener('touchend', handleTouchEnd);
  }
});
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition-all duration-200"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition-all duration-200"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div
        v-if="open"
        class="fixed inset-0 z-[200] bg-black/95 backdrop-blur-xl flex items-center justify-center"
      >
        <!-- 关闭按钮 -->
        <button
          @click="handleClose"
          class="absolute top-4 right-4 md:top-6 md:right-6 z-[210] flex items-center justify-center w-10 h-10 md:w-12 md:h-12 rounded-full bg-white/10 hover:bg-white/20 backdrop-blur-md border border-white/20 transition-all group"
          title="关闭"
        >
          <svg class="w-5 h-5 md:w-6 md:h-6 text-white/80 group-hover:text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>

        <!-- 操作提示 -->
        <div class="absolute top-4 left-4 md:top-6 md:left-6 z-[210] px-3 py-2 md:px-4 md:py-2 rounded-xl bg-black/60 backdrop-blur-md border border-white/10">
          <div class="text-[10px] md:text-xs text-white/70 space-y-0.5 md:space-y-1">
            <div class="hidden md:block">🔍 滚轮缩放</div>
            <div>✌️ 双指缩放</div>
            <div>✋ 单指拖动</div>
            <div>🔄 双击放大/重置</div>
          </div>
        </div>

        <!-- 缩放比例显示 -->
        <div class="absolute bottom-4 left-4 md:bottom-6 md:left-6 z-[210] px-3 py-2 md:px-4 md:py-2 rounded-xl bg-black/60 backdrop-blur-md border border-white/10">
          <div class="text-xs md:text-sm font-bold text-white/90">
            {{ Math.round(scale * 100) }}%
          </div>
        </div>

        <!-- 重置按钮 -->
        <button
          @click="resetView"
          class="absolute bottom-4 right-4 md:bottom-6 md:right-6 z-[210] px-3 py-2 md:px-4 md:py-2 rounded-xl bg-white/10 hover:bg-white/20 backdrop-blur-md border border-white/20 transition-all text-xs md:text-sm font-bold text-white/90"
        >
          重置
        </button>

        <!-- 图片容器 -->
        <div
          ref="containerRef"
          class="relative w-full h-full overflow-hidden cursor-grab active:cursor-grabbing touch-none"
          @wheel.prevent="handleWheel"
          @mousedown="handleMouseDown"
          @touchstart.prevent="handleTouchStart"
          @touchmove.prevent="handleTouchMove"
          @touchend="handleTouchEnd"
          @dblclick="handleDoubleClick"
        >
          <Transition
            enter-active-class="transition-all duration-300"
            enter-from-class="opacity-0 scale-95"
            enter-to-class="opacity-100 scale-100"
            leave-active-class="transition-all duration-200"
            leave-from-class="opacity-100 scale-100"
            leave-to-class="opacity-0 scale-95"
          >
            <img
              v-if="open"
              ref="imageRef"
              :src="src"
              class="absolute top-1/2 left-1/2 max-w-[90vw] max-h-[90vh] object-contain origin-center select-none pointer-events-none"
              :style="{
                transform: `translate(-50%, -50%) translate(${translateX}px, ${translateY}px) scale(${scale})`
              }"
              alt="预览图片"
              draggable="false"
            />
          </Transition>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
