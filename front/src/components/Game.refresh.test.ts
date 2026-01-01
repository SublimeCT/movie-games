import { mount } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import Game from './Game.vue';

const gameDataRef = ref<Record<string, unknown> | null>(null);
const handleGameEnd = vi.fn();

vi.mock('../hooks/useGameState', () => {
  return {
    useGameState: () => ({
      gameData: gameDataRef,
      handleGameEnd,
    }),
  };
});

vi.mock('vue-router', () => {
  return {
    useRouter: () => ({
      push: vi.fn(),
    }),
  };
});

describe('Game refresh recovery', () => {
  afterEach(() => {
    gameDataRef.value = null;
    handleGameEnd.mockClear();
    localStorage.clear();
    sessionStorage.clear();
  });

  it('auto-skips invalid start node after async gameData hydration', async () => {
    localStorage.setItem('mg_current_node', 'start');

    const wrapper = mount(Game);

    expect(localStorage.getItem('mg_current_node')).toBe('start');

    gameDataRef.value = {
      title: 't',
      projectId: 'p',
      version: '1',
      owner: 'o',
      meta: {
        logline: '',
        synopsis: '',
        targetRuntimeMinutes: 0,
        genre: '',
        language: 'zh-CN',
      },
      nodes: {
        start: { id: 'start' },
        '1': {
          id: '1',
          content: 'Hello',
          choices: [{ text: 'Go', nextNodeId: '2' }],
        },
        '2': { id: '2', content: 'End', choices: [] },
      },
      endings: {},
      characters: {},
      provenance: { createdBy: 't', createdAt: 't' },
    };

    await wrapper.vm.$nextTick();
    await wrapper.vm.$nextTick();

    expect(localStorage.getItem('mg_current_node')).toBe('1');
    expect(wrapper.findAll('button.cinematic-choice').length).toBeGreaterThan(
      0,
    );
    expect(wrapper.text()).toContain('Hello');
    expect(handleGameEnd).not.toHaveBeenCalled();
  });
});
