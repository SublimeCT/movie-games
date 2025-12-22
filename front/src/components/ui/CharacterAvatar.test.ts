import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import CharacterAvatar from './CharacterAvatar.vue';

describe('CharacterAvatar', () => {
  it('renders svg fallback when avatarPath is missing', () => {
    const wrapper = mount(CharacterAvatar, {
      props: {
        name: 'Alice',
      },
    });

    expect(wrapper.find('img').exists()).toBe(false);
    expect(wrapper.find('svg').exists()).toBe(true);
  });

  it('renders image when avatarPath exists', () => {
    const wrapper = mount(CharacterAvatar, {
      props: {
        name: 'Alice',
        avatarPath: 'data:image/png;base64,AAA',
      },
    });

    const img = wrapper.find('img');
    expect(img.exists()).toBe(true);
    expect(img.attributes('src')).toBe('data:image/png;base64,AAA');
    expect(wrapper.find('svg').exists()).toBe(false);
  });
});
