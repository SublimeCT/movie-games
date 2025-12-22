import path from 'node:path';
import vue from '@vitejs/plugin-vue';
import { defineConfig } from 'vitest/config';

/**
 * Vite + Vitest 配置。
 * 这里显式设置测试与钩子超时时间，避免测试在异常情况下无限挂起。
 */
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:35275',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, ''),
      },
    },
  },
  test: {
    environment: 'happy-dom',
    testTimeout: 20_000,
    hookTimeout: 20_000,
  },
});
