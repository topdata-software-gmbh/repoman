import { computed, type Ref } from 'vue';
import { useLanguage } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { en, type MessageKey } from './messages.en';
import { de } from './messages.de';

export type { MessageKey };

export type Language = 'en' | 'de';
export const LANGUAGES: readonly Language[] = ['de', 'en'];

export type MessageParams = Record<string, string | number>;
export type MessageValue = string | ((params: MessageParams) => string);
type PluralBase<K extends string> = K extends `${infer Base}.one` ? Base : never;
export type PluralKey = PluralBase<MessageKey>;

const catalogues: Record<Language, Record<MessageKey, MessageValue>> = { en, de };

const state = useLanguage({ defaultLanguage: 'de', supported: LANGUAGES });

export const language: Readonly<Ref<Language>> = computed(() =>
  state.language.value === 'de' ? 'de' : 'en',
);

export function setLanguage(next: Language): void {
  state.setLanguage(next);
}

function interpolate(template: string, params: MessageParams): string {
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    Object.prototype.hasOwnProperty.call(params, name)
      ? String(params[name])
      : match,
  );
}

export function t(key: MessageKey, params: MessageParams = {}): string {
  const template = catalogues[language.value][key] ?? en[key];
  if (template === undefined) {
    if (import.meta.env.DEV) {
      console.warn(`[i18n] Missing message "${key}" for language "${language.value}".`);
    }
    return key;
  }
  return interpolate(typeof template === 'function' ? template(params) : template, params);
}

export function tPlural(
  baseKey: PluralKey,
  count: number,
  params: MessageParams = {},
): string {
  const suffix = count === 1 ? 'one' : 'other';
  return t(`${baseKey}.${suffix}` as MessageKey, { ...params, count });
}
