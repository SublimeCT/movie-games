import { createRouter, createWebHistory } from 'vue-router';
import EndingPage from '../components/Ending.vue';
import Game from '../components/Game.vue';
import Home from '../components/Home.vue';

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: Home,
    },
    {
      path: '/generating',
      name: 'generating',
      component: () => import('../components/Generating.vue'),
    },
    {
      path: '/game',
      name: 'game',
      component: Game,
    },
    {
      path: '/ending',
      name: 'ending',
      component: EndingPage,
    },
    {
      path: '/play/:id',
      name: 'play',
      component: () => import('../components/Play.vue'),
    },
    {
      path: '/records',
      name: 'records',
      component: () => import('../components/Records.vue'),
    },
    {
      path: '/design',
      name: 'design',
      component: () => import('../components/Designer.vue'),
    },
  ],
});

export default router;
