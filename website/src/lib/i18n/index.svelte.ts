import { ko } from './ko';
import { en } from './en';

export type Language = 'ko' | 'en';

// Use a simple reactive object in Svelte 5
export const i18n = $state({
  lang: 'ko' as Language,
  get t() {
    return this.lang === 'ko' ? ko : en;
  },
  toggle() {
    this.lang = this.lang === 'ko' ? 'en' : 'ko';
  }
});
