import { createApp } from 'vue';
import './style.css';
import App from './App.vue';
import router from './router';

/**
 * Microsoft Clarity Analytics（仅生产环境启用）。
 */
if (import.meta.env.PROD) {
  // biome-ignore lint/suspicious/noExplicitAny: Clarity analytics setup
  ((c: any, l: any, a: any, r: any, i: any) => {
    c[a] =
      c[a] ||
      ((...args: unknown[]) => {
        if (!c[a].q) c[a].q = [];
        c[a].q.push(args);
      });

    const tag = l.createElement(r);
    tag.async = 1;
    tag.src = `https://www.clarity.ms/tag/${i}`;

    const first = l.getElementsByTagName(r)[0];
    if (first?.parentNode) {
      first.parentNode.insertBefore(tag, first);
    }
  })(window, document, 'clarity', 'script', 'up07r30a67');
}

createApp(App).use(router).mount('#app');
