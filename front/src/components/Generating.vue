<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { ApiError, type GenerateRequest, generateGame } from '../api';
import { useGameState } from '../hooks/useGameState';
import CinematicLoader from './ui/CinematicLoader.vue';
import { FluidCursor } from './ui/fluid-cursor';

const router = useRouter();

// 使用 hook 获取游戏开始方法
const { handleGameStart } = useGameState();

const isLoading = ref(true);
const error = ref('');
const countdown = ref(5);
const errorCode = ref('');
let countdownInterval: ReturnType<typeof setInterval> | null = null;

// Auto redirect to home on error
const startAutoRedirect = () => {
  countdown.value = 5;
  countdownInterval = setInterval(() => {
    countdown.value--;
    if (countdown.value <= 0) {
      if (countdownInterval) clearInterval(countdownInterval);
      handleGoBack();
    }
  }, 1000);
};

const handleGoBack = () => {
  if (countdownInterval) clearInterval(countdownInterval);
  // Store error info for Home.vue to display
  if (error.value) {
    sessionStorage.setItem(
      'mg_last_error',
      JSON.stringify({
        message: error.value,
        code: errorCode.value,
        timestamp: Date.now(),
      }),
    );
  }
  router.push('/');
};

onMounted(async () => {
  // Read parameters from localStorage
  const paramsStr = localStorage.getItem('mg_generate_params');
  if (!paramsStr) {
    error.value = '缺少生成参数';
    errorCode.value = 'MISSING_PARAMS';
    isLoading.value = false;
    startAutoRedirect();
    return;
  }

  try {
    const params = JSON.parse(paramsStr);

    /**
     * 兜底从本地存储读取角色阵容，避免生成请求遗漏角色信息。
     */
    const readCharactersFromStorage = () => {
      try {
        const raw = localStorage.getItem('mg_characters');
        const v = raw ? (JSON.parse(raw) as unknown) : [];
        return Array.isArray(v) ? v : [];
      } catch {
        return [];
      }
    };

    const characters =
      Array.isArray(params.characters) && params.characters.length > 0
        ? params.characters
        : readCharactersFromStorage();

    const request: GenerateRequest = {
      mode: 'wizard',
      theme: params.theme,
      synopsis: params.synopsis,
      genre: params.genre,
      characters:
        Array.isArray(characters) && characters.length > 0
          ? characters
          : [
              {
                name: '主角',
                description: '故事的核心人物',
                gender: '男',
                isMain: true,
              },
            ],
      language: params.language,
      size: params.size,
      apiKey: params.apiKey,
      baseUrl: params.baseUrl,
      model: params.model,
    };

    // Clear the stored params after reading
    localStorage.removeItem('mg_generate_params');

    // Generate the game
    const data = await generateGame(request);

    // 调用 handleGameStart 更新全局状态
    handleGameStart(data);
  } catch (e) {
    if (e instanceof ApiError) {
      // Show detailed error information
      if (e.code === 'API_KEY_REQUIRED') {
        error.value = '需要配置 API Key，请使用您自己的 API Key';
        errorCode.value = 'API_KEY_REQUIRED';
      } else if (e.code === 'TOO_MANY_REQUESTS') {
        error.value = 'API 额度已用完，请使用您自己的 API Key';
        errorCode.value = 'TOO_MANY_REQUESTS';
      } else if (e.code === 'INVALID_BASE_URL') {
        error.value = 'API 地址无效，请检查设置';
        errorCode.value = 'INVALID_BASE_URL';
      } else {
        // Show the actual error message from backend
        error.value = e.message || `生成失败 (${e.status})`;
        errorCode.value = e.code || 'UNKNOWN';
      }
    } else {
      error.value = e instanceof Error ? e.message : '生成失败，请重试';
      errorCode.value = 'UNKNOWN';
    }
    isLoading.value = false;
    startAutoRedirect();
  }
});

onUnmounted(() => {
  if (countdownInterval) clearInterval(countdownInterval);
});
</script>

<template>
  <div class="generating-page">
    <CinematicLoader v-if="isLoading" />
    <FluidCursor v-if="isLoading" class="z-[55]" />

    <!-- Error State -->
    <div v-if="error" class="error-overlay">
      <div class="error-card">
        <div class="error-icon">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
        </div>
        <p class="error-message">{{ error }}</p>
        <div v-if="errorCode === 'API_KEY_REQUIRED' || errorCode === 'TOO_MANY_REQUESTS'" class="error-tip">
          请在首页右上角「连接设置」中配置您自己的 API Key
        </div>
        <div class="countdown">
          <span class="countdown-number">{{ countdown }}</span> 秒后自动返回首页
        </div>
        <button @click="handleGoBack" class="back-button">
          立即返回
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.generating-page {
  position: relative;
  height: 100vh;
  width: 100%;
}

.error-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.85);
  backdrop-filter: blur(12px);
  animation: fadeIn 0.3s ease-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.error-card {
  background: rgba(239, 68, 68, 0.08);
  border: 1px solid rgba(239, 68, 68, 0.3);
  border-radius: 1.25rem;
  padding: 2.5rem 2rem;
  max-width: 32rem;
  text-align: center;
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
  animation: slideUp 0.3s ease-out;
}

@keyframes slideUp {
  from {
    transform: translateY(20px);
    opacity: 0;
  }
  to {
    transform: translateY(0);
    opacity: 1;
  }
}

.error-icon {
  width: 3.5rem;
  height: 3.5rem;
  margin: 0 auto 1.25rem;
  border-radius: 50%;
  background: rgba(239, 68, 68, 0.15);
  display: flex;
  align-items: center;
  justify-content: center;
}

.error-icon svg {
  width: 2rem;
  height: 2rem;
  color: #f87171;
}

.error-message {
  color: #fca5a5;
  font-size: 1.125rem;
  margin-bottom: 1rem;
  line-height: 1.6;
}

.error-tip {
  color: #fbbf24;
  font-size: 0.875rem;
  margin-bottom: 1.5rem;
  padding: 0.75rem 1rem;
  background: rgba(251, 191, 36, 0.1);
  border-radius: 0.5rem;
  border: 1px solid rgba(251, 191, 36, 0.2);
}

.countdown {
  color: rgba(255, 255, 255, 0.6);
  font-size: 0.875rem;
  margin-bottom: 1.5rem;
}

.countdown-number {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.5rem;
  height: 1.5rem;
  padding: 0 0.375rem;
  background: rgba(239, 68, 68, 0.2);
  border-radius: 0.375rem;
  color: #f87171;
  font-weight: 600;
  font-size: 1rem;
  margin: 0 0.125rem;
}

.back-button {
  padding: 0.75rem 2rem;
  border-radius: 0.75rem;
  background: rgba(255, 255, 255, 0.1);
  color: white;
  font-weight: 500;
  font-size: 0.9375rem;
  transition: all 0.2s;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.back-button:hover {
  background: rgba(255, 255, 255, 0.15);
  border-color: rgba(255, 255, 255, 0.2);
}
</style>
