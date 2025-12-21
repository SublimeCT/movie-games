import { createRouter, createWebHistory } from 'vue-router';
import Home from '../components/Home.vue';
import Game from '../components/Game.vue';
import EndingPage from '../components/Ending.vue';

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: Home,
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
  ],
});

export default router;
