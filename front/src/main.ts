import { createApp } from 'vue';
import './style.css';
import App from './App.vue';
import router from './router';

// Microsoft Clarity Analytics - Production only
if (import.meta.env.PROD) {
  // biome-ignore lint/suspicious/noExplicitAny: Clarity analytics setup
  ((c: any, l: any, a: any, r: any, i: any, t?: any, y?: any) => {
    c[a] = c[a] || function () {
      // biome-ignore lint/suspicious/noExplicitAny: Clarity analytics setup
      (c[a].q = c[a].q || []).push(arguments);
    };
    t = l.createElement(r);
    t.async = 1;
    t.src = `https://www.clarity.ms/tag/${i}`;
    y = l.getElementsByTagName(r)[0];
    y.parentNode.insertBefore(t, y);
  })(window, document, 'clarity', 'script', 'up07r30a67');
}

createApp(App).use(router).mount('#app');
