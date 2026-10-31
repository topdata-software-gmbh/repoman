import { createApp } from 'vue';
import { createPinia } from 'pinia';
import { THEME_STORAGE_KEY } from '@topdata-software-gmbh/ui-kit-web-vue3';
import '@topdata-software-gmbh/design-tokens/tokens.css';
import '@topdata-software-gmbh/ui-kit-web-vue3/styles.css';
import App from './App.vue';
import './lib/i18n';
import './style.css';

try {
  if (!window.localStorage.getItem(THEME_STORAGE_KEY)) {
    window.localStorage.setItem(THEME_STORAGE_KEY, 'light');
  }
} catch {
  /* Storage can be unavailable in hardened webviews. */
}

createApp(App).use(createPinia()).mount('#app');
