import { mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it } from 'vitest';
import ReviewImageDiff from '../reviews/ReviewImageDiff.vue';
import { setLanguage } from '../../lib/i18n';
import type { FilePreview, ImagePreview } from '../../types/domain';

function preview(image: ImagePreview): FilePreview {
  return {
    path: 'assets/logo.png',
    oldPath: null,
    status: 'modified',
    kind: 'image',
    whitespaceOnly: false,
    addedLines: 0,
    removedLines: 0,
    notice: image.state === 'ready' ? null : (image.state === 'too-large' ? 'too-large' : 'corrupt'),
    image,
  };
}

const side = (bytes: number, width = 800, height = 400) => ({
  dataUrl: `data:image/png;base64,${bytes}`,
  width,
  height,
  bytes,
});

describe('ReviewImageDiff', () => {
  beforeEach(() => {
    setLanguage('en');
  });

  it('shows before and after images with dimensions and byte sizes', () => {
    const wrapper = mount(ReviewImageDiff, {
      props: {
        preview: preview({
          state: 'ready',
          old: side(51200, 800, 400),
          new: side(53000, 800, 420),
        }),
      },
    });

    const images = wrapper.findAll('img');
    expect(images).toHaveLength(2);
    expect(images[0].attributes('src')).toContain('data:image/png;base64,');
    expect(wrapper.text()).toContain('Before');
    expect(wrapper.text()).toContain('After');
    expect(wrapper.text()).toContain('800 × 400 px');
    expect(wrapper.text()).toContain('800 × 420 px');
    expect(wrapper.text()).toContain('50 KB');
    expect(wrapper.text()).toContain('52 KB');
  });

  it('shows only the available side for an added file', () => {
    const wrapper = mount(ReviewImageDiff, {
      props: {
        preview: preview({ state: 'ready', old: null, new: side(1024, 64, 64) }),
      },
    });

    expect(wrapper.findAll('img')).toHaveLength(1);
    expect(wrapper.text()).toContain('After');
    expect(wrapper.text()).not.toContain('Before');
  });

  it('replaces an oversized image with a plain notice', () => {
    const wrapper = mount(ReviewImageDiff, {
      props: {
        preview: preview({ state: 'too-large', old: null, new: null }),
      },
    });

    expect(wrapper.findAll('img')).toHaveLength(0);
    expect(wrapper.text()).toContain('too large');
  });

  it('replaces a corrupt image with a plain notice', () => {
    const wrapper = mount(ReviewImageDiff, {
      props: {
        preview: preview({ state: 'corrupt', old: null, new: null }),
      },
    });
    expect(wrapper.findAll('img')).toHaveLength(0);
    expect(wrapper.text()).toContain('cannot be displayed');
  });
});
