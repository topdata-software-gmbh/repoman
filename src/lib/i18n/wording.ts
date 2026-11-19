import { ref, type Ref } from 'vue';

export type Wording = 'plain' | 'git';

export const WORDINGS: readonly Wording[] = ['plain', 'git'];
export const WORDING_STORAGE_KEY = 'topdata.wording.v1';

export interface WordingStorage {
  get(key: string): string | null;
  set(key: string, value: string): void;
}

export interface UseWordingOptions {
  defaultWording?: Wording;
  supported?: readonly Wording[];
  storage?: WordingStorage;
}

export interface UseWording {
  wording: Readonly<Ref<Wording>>;
  setWording(next: Wording): void;
}

function isSupportedWording(
  value: unknown,
  supported: readonly Wording[],
): value is Wording {
  return typeof value === 'string' && (supported as readonly string[]).includes(value);
}

function browserStorage(): WordingStorage | undefined {
  if (typeof window === 'undefined') return undefined;
  try {
    const storage = window.localStorage;
    if (!storage) return undefined;
    return {
      get: (key) => storage.getItem(key),
      set: (key, value) => storage.setItem(key, value),
    };
  } catch {
    /* Storage can be disabled or hardened; the in-memory value stays usable. */
    return undefined;
  }
}

/**
 * Creates a wording preference. The factory exists so tests can inject storage;
 * the app uses the module singleton below so every component sees one ref.
 */
export function useWording(options: UseWordingOptions = {}): UseWording {
  const supported = options.supported ?? WORDINGS;
  const defaultWording = isSupportedWording(options.defaultWording, supported)
    ? options.defaultWording
    : 'plain';
  const storage = options.storage ?? browserStorage();

  function readStored(): Wording {
    if (!storage) return defaultWording;
    try {
      const stored = storage.get(WORDING_STORAGE_KEY);
      return isSupportedWording(stored, supported) ? stored : defaultWording;
    } catch {
      return defaultWording;
    }
  }

  const state = ref<Wording>(readStored());

  function setWording(next: Wording): void {
    if (!isSupportedWording(next, supported)) {
      if (import.meta.env.DEV) {
        console.warn(`[wording] Ignoring unsupported wording "${String(next)}".`);
      }
      return;
    }

    state.value = next;
    if (!storage) return;
    try {
      storage.set(WORDING_STORAGE_KEY, next);
    } catch {
      /* Storage failures must never break the preference or the app startup. */
    }
  }

  return { wording: state, setWording };
}

const singleton = useWording();

export const wording: Readonly<Ref<Wording>> = singleton.wording;
export const setWording = singleton.setWording;
