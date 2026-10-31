---
filename: "_ai/backlog/active/261031_2243__IMPLEMENTATION_PLAN__language-switcher-and-german-ui.md"
title: "Implementation Plan: Language switcher and German UI (design system + repo-manager)"
createdAt: 2026-10-31 22:43
updatedAt: 2026-10-31 22:43
status: draft
priority: high
tags: [i18n, german, vue, design-system, accessibility, ui]
estimatedComplexity: complex
documentRevision: 1
sourceSpec: ""
sourceHandoff: ""
sourceAdrs: ["/topdata/topdata-design-system/_ai/technical_decisions/261030_1838__ADR__web-ui-kit-vue3-token-only-styling.md", "/topdata/topdata-design-system/_ai/technical_decisions/261029_0240__ADR__ui-component-registry.md"]
documentType: IMPLEMENTATION_PLAN
---

# Implementation Plan: Language switcher and German UI (design system + repo-manager)

Sources used: this conversation (design discussion), `_ai/technical_decisions/` in
`/topdata/repoman`, and `_ai/technical_decisions/` in `/topdata/topdata-design-system`
— specifically `261030-1` (token-only styling, global `.td-*` class contract) and
`261029-1` (UI component registry, one kit per surface). No feature spec and no
design handoff exist, so `sourceSpec` and `sourceHandoff` are empty.

---

## 1. Problem

Sandra needs a German user interface. Today every string in the Vue frontend is a
hard-coded English literal: navigation, toolbar, tables, empty states, onboarding,
modals, window confirmations, `aria-label`s, `title`s, placeholders, and dynamic
copy produced by the Pinia store. There is no way to switch language, no persisted
preference, and no shared pattern for other TopData Vue apps to follow.

`_ai/conventions.md` already requires externalised copy with interpolation and
pluralisation (`ui:i18n`), so the current state violates a project rule. The shared
design system (`/topdata/topdata-design-system`) has `useTheme` but no language
primitive and no switcher, so every consumer would otherwise invent its own.

The requested outcome: a language switcher in the app, German as the default, all
Vue-owned copy translated, and the reusable composable/component added to the
design system.

## 2. Executive summary

Add two dictionary-free primitives to `@topdata-software-gmbh/ui-kit-web-vue3`:

1. `useLanguage` — reads/writes `localStorage` key `topdata.language.v1`, accepts
   `defaultLanguage`, `supported`, and a custom storage adapter, and keeps
   `document.documentElement.lang` in sync. It mirrors `useTheme` and owns no
   catalogue.
2. `TdLanguageSwitcher` — a compact ghost button showing the active code (`DE`),
   opening a `TdMenu` of endonyms with the active one checked.

Repo-manager then adds one app-owned translation runtime (`src/lib/i18n/`) with
typed English and German catalogues, a `t()` / `tPlural()` helper, and a single
`useLanguage` instance. German is the shipped default (`de`), English is selectable,
and the choice persists per browser. Every Vue-rendered string — including
attributes, confirms, validation, and store-generated insights/toasts — is
externalised. The switcher sits in the sidebar footer beside the theme toggle.

The Rust backend, Git transcript from `src-tauri/macros/git-actions.yaml`, and
`AppConfig` persistence are deliberately untouched. This keeps the change
frontend-only and prevents the YAML and translation catalogues from drifting.

## 3. Design Decisions & Rejected Alternatives

| # | Decision | Rationale / alternative dropped |
|---|----------|--------------------------------|
| D1 | Typed app-owned catalogue (`messages.en.ts` + `messages.de.ts`), no `vue-i18n` | The app has a bounded vocabulary and two locales. TypeScript enforces key parity at build time; a runtime dependency and plugin wiring buy nothing yet. `vue-i18n` would be reconsidered for many locales or translator tooling. |
| D2 | The kit stays dictionary-free; apps own copy | Keeps the kit presentational and avoids bundling German/English into every consumer. Shipping kit dictionaries was considered and rejected for this increment. |
| D3 | `useLanguage` per-call ref, not a hidden singleton | Mirrors `useTheme`, keeps tests isolated, and lets the app own exactly one instance in `src/lib/i18n/index.ts`. A module-level singleton in the kit was rejected as surprising global state. |
| D4 | Compact menu switcher (`TdLanguageSwitcher`) | Fits the sidebar footer next to the theme/more buttons and scales past two locales. Segmented `EN | DE` was rejected for width and locale-count limits; native `TdSelect` for being visually heavy. |
| D5 | `localStorage` only, key `topdata.language.v1` | Consistent with the theme preference, instant, no backend schema change. Persisting in `AppConfig` was rejected for this pass because it requires Rust model/migration work and an async write on every switch. |
| D6 | German default (`defaultLanguage: "de"`), English selectable | Sandra gets German immediately. Browser auto-detection was rejected: it makes first-run behaviour depend on OS locale and is harder to test. |
| D7 | Informal German **du** throughout | User decision. Formal **Sie** and impersonal style were considered; **du** matches the friendly plain-language tone of the English copy. |
| D8 | Translate Vue-owned copy only | Backend error details and the literal Git transcript remain English. Threading language through Rust/IPC and adding `title_de`/`comment_de` to `git-actions.yaml` was rejected as a second feature with YAML/backend drift risk. |
| D9 | Existing visible toasts keep their creation language | Resolving `t()` when the toast is pushed avoids storing keys in `Toast` and re-resolving on every switch. Insights and all rendered labels do update instantly because `t()` reads the reactive language ref. |
| D10 | Add one ADR recording the durable boundary | The “kit dictionary-free / app owns catalogues / no `vue-i18n` yet” rule constrains future TopData Vue apps. Recorded in `_ai/technical_decisions/ADR__261031-1__app-owned-translations-and-dictionary-free-kit.md`. |

**No open questions or blockers.** All decisions above were confirmed in this
conversation.

---

## 4. Project environment and conventions

- **Repositories**
  - App: `/topdata/repoman` (Tauri v2 + Vue 3 + Pinia; Rust core shared by desktop and axum web).
  - Kit: `/topdata/topdata-design-system` (pnpm workspace; `packages/tokens`, `packages/ui-kit-web-vue3`, `playground`).
- **Git workflow**: repository-defined. Neither repo has an `AGENTS.md` that requires branches or PRs. Implement every phase on the current branch; **do not create branches and do not open PRs**. Commit only when the user explicitly asks.
- **Vue**: `<script setup lang="ts">`, Composition API, PascalCase files. No raw palette classes or hex colours in app source — `src/lib/__tests__/token-only.test.ts` enforces token-only styling.
- **Kit styling**: component visuals live in global `.td-*` classes under `packages/ui-kit-web-vue3/src/styles/components/*.css`, built only from `--td-*` tokens (ADR `261030-1`). SFCs contain behaviour, slots, typing, and ARIA; no `<style scoped>`.
- **Kit registry**: new kit surface changes require registry/ADR awareness (ADR `261029-1`). This plan adds components to the existing `topdata-vue` kit; no new kit or surface.
- **TypeScript**: run `vue-tsc --noEmit` via each repo's `pnpm typecheck`.
- **Verification commands**:

```bash
# Design system
cd /topdata/topdata-design-system
pnpm --filter @topdata-software-gmbh/ui-kit-web-vue3 test
pnpm --filter @topdata-software-gmbh/ui-kit-web-vue3 typecheck
pnpm --filter @topdata-software-gmbh/ui-kit-web-vue3 build

# App (after the kit build, because packages are link: dependencies)
cd /topdata/repoman
pnpm typecheck
pnpm test
pnpm build
```

- **Link dependency note**: `repoman/package.json` links `../topdata-design-system/packages/*`. Rebuild the kit before app typecheck when the kit's `dist/` types change.
- **New files introduced**:
  - Kit: `src/composables/useLanguage.ts`, `src/composables/useLanguage.test.ts`, `src/components/TdLanguageSwitcher.vue`, `src/components/TdLanguageSwitcher.test.ts`, `src/styles/components/language-switcher.css`, `_ai/specs/SPEC__261031-1__td-language-switcher.md`.
  - App: `src/lib/i18n/index.ts`, `src/lib/i18n/messages.en.ts`, `src/lib/i18n/messages.de.ts`, `src/lib/__tests__/i18n.test.ts`.
  - Neither repo introduces build artifacts or directories that require `.gitignore` changes; Phase 12 verifies this.
- **SOLID**: the language runtime (persistence/`<html lang>`) is separated from the catalogue (copy) and from the presentational switcher. Components depend on the `t()` abstraction, not on a specific catalogue.

---

## 5. Requirements traceability

The conversation is the source of requirements (no spec file). Each requirement must
be satisfied by at least one phase.

| ID | Requirement | Satisfied in |
|----|-------------|--------------|
| R1 | Kit exposes `useLanguage` with storage, default, supported-list guard, and `<html lang>` sync | Phase 1 |
| R2 | Kit exposes `TdLanguageSwitcher` as a compact menu with accessible name and active marking | Phase 2 |
| R3 | Kit ships spec, README, playground, and tests for both additions | Phase 3 |
| R4 | App ships typed English and German catalogues with interpolation and pluralisation | Phases 4–5 |
| R5 | German is the default; English is selectable; choice persists across reloads | Phase 6 |
| R6 | Every Vue-owned visible string is translated (text, `title`, `aria-label`, placeholder, confirm) | Phases 7–9 |
| R7 | Store-generated insights, toast titles/messages, and validation errors use `t()`/`tPlural()` | Phase 9 |
| R8 | Existing tests keep passing; new tests cover language runtime, catalogue, switcher, and German default | Phases 1, 2, 10 |
| R9 | Rust, Git transcript, `AppConfig` persistence, browser detection, and kit-internal labels stay out of scope | Phases 6, 9, 11 |
| R10 | Both repos pass typecheck, tests, and build; docs/changelog/ADR updated | Phases 3, 11, 12 |

---

## 6. Phase 1 — Kit: `useLanguage` composable

**Goal:** provide a theme-like language preference primitive with persistence and
`<html lang>` sync.

### Files

- [NEW FILE] `packages/ui-kit-web-vue3/src/composables/useLanguage.ts`
- [NEW FILE] `packages/ui-kit-web-vue3/src/composables/useLanguage.test.ts`

### Implementation

```ts
// [NEW FILE] packages/ui-kit-web-vue3/src/composables/useLanguage.ts
import { computed, ref, type Ref } from "vue";

export type LanguageCode = string;

export const LANGUAGE_STORAGE_KEY = "topdata.language.v1";

export interface LanguageStorage {
  get(key: string): string | null;
  set(key: string, value: string): void;
}

export interface UseLanguageOptions {
  /** Language used when nothing valid is stored. Defaults to `"en"`. */
  defaultLanguage?: LanguageCode;
  /** Restricts stored and set values. Defaults to `[defaultLanguage]`. */
  supported?: readonly LanguageCode[];
  /** Persistence adapter; defaults to `localStorage`. */
  storage?: LanguageStorage;
}

export interface UseLanguage {
  language: Readonly<Ref<LanguageCode>>;
  setLanguage(code: LanguageCode): void;
}

const DEFAULT_LANGUAGE = "en";

const browserStorage: LanguageStorage = {
  get(key) {
    try {
      return typeof window === "undefined" ? null : window.localStorage.getItem(key);
    } catch {
      return null;
    }
  },
  set(key, value) {
    try {
      if (typeof window !== "undefined") window.localStorage.setItem(key, value);
    } catch {
      /* storage can be unavailable in hardened webviews */
    }
  },
};

function resolveSupported(options: UseLanguageOptions): readonly LanguageCode[] {
  if (options.supported && options.supported.length > 0) return options.supported;
  return [options.defaultLanguage ?? DEFAULT_LANGUAGE];
}

function resolveFallback(
  options: UseLanguageOptions,
  supported: readonly LanguageCode[],
): LanguageCode {
  const preferred = options.defaultLanguage ?? DEFAULT_LANGUAGE;
  return supported.includes(preferred) ? preferred : supported[0] ?? DEFAULT_LANGUAGE;
}

function applyLanguage(code: LanguageCode): void {
  if (typeof document !== "undefined") {
    document.documentElement.lang = code;
  }
}

export function useLanguage(options: UseLanguageOptions = {}): UseLanguage {
  const storage = options.storage ?? browserStorage;
  const supported = resolveSupported(options);
  const fallback = resolveFallback(options, supported);
  const stored = storage.get(LANGUAGE_STORAGE_KEY);
  const initial = stored !== null && supported.includes(stored) ? stored : fallback;

  const language = ref<LanguageCode>(initial);
  applyLanguage(initial);

  function setLanguage(next: LanguageCode): void {
    if (!supported.includes(next)) {
      if (import.meta.env.DEV) {
        console.warn(
          `[useLanguage] Unsupported language "${next}". Supported: ${supported.join(", ")}.`,
        );
      }
      return;
    }

    language.value = next;
    storage.set(LANGUAGE_STORAGE_KEY, next);
    applyLanguage(next);
  }

  return { language: computed(() => language.value), setLanguage };
}
```

### Tests

```ts
// [NEW FILE] packages/ui-kit-web-vue3/src/composables/useLanguage.test.ts
import { effectScope } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { LANGUAGE_STORAGE_KEY, useLanguage } from "./useLanguage";

function inScope<T>(factory: () => T): T {
  const scope = effectScope();
  const result = scope.run(factory);
  if (result === undefined) throw new Error("useLanguage did not run");
  return result;
}

beforeEach(() => {
  window.localStorage.clear();
  document.documentElement.removeAttribute("lang");
});

describe("useLanguage", () => {
  it("applies the default language and sets html lang", () => {
    const state = inScope(() => useLanguage({ defaultLanguage: "de" }));

    expect(state.language.value).toBe("de");
    expect(document.documentElement.lang).toBe("de");
  });

  it("uses a stored supported language over the default", () => {
    window.localStorage.setItem(LANGUAGE_STORAGE_KEY, "en");

    const state = inScope(() =>
      useLanguage({ defaultLanguage: "de", supported: ["de", "en"] }),
    );

    expect(state.language.value).toBe("en");
    expect(document.documentElement.lang).toBe("en");
  });

  it("falls back when the stored value is unsupported", () => {
    window.localStorage.setItem(LANGUAGE_STORAGE_KEY, "fr");

    const state = inScope(() =>
      useLanguage({ defaultLanguage: "de", supported: ["de", "en"] }),
    );

    expect(state.language.value).toBe("de");
  });

  it("persists and applies a supported change", () => {
    const state = inScope(() =>
      useLanguage({ defaultLanguage: "de", supported: ["de", "en"] }),
    );

    state.setLanguage("en");

    expect(state.language.value).toBe("en");
    expect(document.documentElement.lang).toBe("en");
    expect(window.localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("en");
  });

  it("ignores an unsupported change", () => {
    const state = inScope(() =>
      useLanguage({ defaultLanguage: "de", supported: ["de", "en"] }),
    );
    state.setLanguage("fr");

    expect(state.language.value).toBe("de");
    expect(window.localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBeNull();
  });

  it("accepts a custom storage adapter", () => {
    const set = vi.fn();
    const storage = { get: vi.fn().mockReturnValue("en"), set };

    const state = inScope(() =>
      useLanguage({ defaultLanguage: "de", supported: ["de", "en"], storage }),
    );
    state.setLanguage("de");

    expect(state.language.value).toBe("de");
    expect(set).toHaveBeenCalledWith(LANGUAGE_STORAGE_KEY, "de");
    expect(window.localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBeNull();
  });
});
```

### Verification

```bash
pnpm --filter @topdata-software-gmbh/ui-kit-web-vue3 test -- useLanguage
pnpm --filter @topdata-software-gmbh/ui-kit-web-vue3 typecheck
```

---

## 7. Phase 2 — Kit: `TdLanguageSwitcher` component

**Goal:** a presentational compact menu switcher that fits the sidebar and is
accessible by keyboard and screen reader.

### Types

```ts
// [MODIFY] packages/ui-kit-web-vue3/src/types.ts — append
export interface TdLanguageOption {
  value: string;
  label: string;
  short?: string;
  disabled?: boolean;
}
```

### Component

```vue
<!-- [NEW FILE] packages/ui-kit-web-vue3/src/components/TdLanguageSwitcher.vue -->
<script setup lang="ts">
import { computed } from "vue";
import TdButton from "./TdButton.vue";
import TdMenu from "./TdMenu.vue";
import TdMenuItem from "./TdMenuItem.vue";
import type { TdLanguageOption } from "../types";

const DEFAULT_LANGUAGES: TdLanguageOption[] = [
  { value: "en", label: "English", short: "EN" },
  { value: "de", label: "Deutsch", short: "DE" },
];

const props = withDefaults(
  defineProps<{
    modelValue: string;
    languages?: TdLanguageOption[];
    ariaLabel?: string;
    size?: "sm" | "md";
  }>(),
  {
    languages: () => DEFAULT_LANGUAGES,
    ariaLabel: "Change language",
    size: "sm",
  },
);

const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const active = computed(
  () =>
    props.languages.find((language) => language.value === props.modelValue) ??
    props.languages[0],
);

const triggerCode = computed(() => {
  if (!active.value) return props.modelValue.toUpperCase();
  return active.value.short ?? active.value.value.toUpperCase();
});

const triggerLabel = computed(
  () => `${props.ariaLabel}: ${active.value?.label ?? props.modelValue}`,
);

function select(value: string): void {
  if (value !== props.modelValue) emit("update:modelValue", value);
}
</script>

<template>
  <TdMenu align="end" :aria-label="ariaLabel">
    <template #trigger="{ open }">
      <TdButton
        variant="ghost"
        :size="size"
        class="td-lang-switcher"
        :aria-label="triggerLabel"
        aria-haspopup="menu"
        :aria-expanded="open"
      >
        <template #icon-left>
          <span class="td-lang-switcher__code">{{ triggerCode }}</span>
        </template>
      </TdButton>
    </template>
    <TdMenuItem
      v-for="option in languages"
      :key="option.value"
      :disabled="option.disabled"
      @select="select(option.value)"
    >
      {{ option.label }}<span
        v-if="option.value === modelValue"
        class="td-lang-switcher__check"
        aria-hidden="true"
      > ✓</span>
    </TdMenuItem>
  </TdMenu>
</template>
```

### Styles

```css
/* [NEW FILE] packages/ui-kit-web-vue3/src/styles/components/language-switcher.css */
.td-lang-switcher__code {
  font-family: var(--td-font-mono);
  font-size: var(--td-text-xs);
  letter-spacing: 0.04em;
}

.td-lang-switcher__check {
  color: var(--td-color-primary);
}
```

### Exports

```ts
// [MODIFY] packages/ui-kit-web-vue3/src/index.ts
import "./styles/components/language-switcher.css";

export { default as TdLanguageSwitcher } from "./components/TdLanguageSwitcher.vue";

export { useLanguage, LANGUAGE_STORAGE_KEY } from "./composables/useLanguage";
export type {
  LanguageCode,
  LanguageStorage,
  UseLanguage,
  UseLanguageOptions,
} from "./composables/useLanguage";
export type { TdLanguageOption } from "./types";
```

### Tests

```ts
// [NEW FILE] packages/ui-kit-web-vue3/src/components/TdLanguageSwitcher.test.ts
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { afterEach, describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../tests/setup";
import TdLanguageSwitcher from "./TdLanguageSwitcher.vue";
import source from "../styles/components/language-switcher.css?raw";

async function settle(): Promise<void> {
  await nextTick();
  await nextTick();
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("TdLanguageSwitcher", () => {
  it("shows the active short code and an accessible name", () => {
    const wrapper = mount(TdLanguageSwitcher, {
      props: { modelValue: "de", ariaLabel: "Sprache ändern" },
    });

    expect(wrapper.find(".td-lang-switcher__code").text()).toBe("DE");
    expect(wrapper.find("button").attributes("aria-label")).toBe(
      "Sprache ändern: Deutsch",
    );
  });

  it("opens a menu of endonyms and marks the active one", async () => {
    const wrapper = mount(TdLanguageSwitcher, {
      props: { modelValue: "de" },
      attachTo: document.body,
    });
    await wrapper.find("button").trigger("click");
    await settle();

    const items = Array.from(document.body.querySelectorAll('[role="menuitem"]'));
    expect(items.map((item) => item.textContent?.trim())).toEqual([
      "English",
      "Deutsch ✓",
    ]);
    wrapper.unmount();
  });

  it("emits the selected language", async () => {
    const wrapper = mount(TdLanguageSwitcher, {
      props: { modelValue: "de" },
      attachTo: document.body,
    });
    await wrapper.find("button").trigger("click");
    await settle();

    (document.body.querySelectorAll('[role="menuitem"]')[0] as HTMLElement).click();
    await settle();

    expect(wrapper.emitted("update:modelValue")).toEqual([["en"]]);
    wrapper.unmount();
  });

  it("does not emit when the active language is selected again", async () => {
    const wrapper = mount(TdLanguageSwitcher, {
      props: { modelValue: "de" },
      attachTo: document.body,
    });
    await wrapper.find("button").trigger("click");
    await settle();

    (document.body.querySelectorAll('[role="menuitem"]')[1] as HTMLElement).click();
    await settle();

    expect(wrapper.emitted("update:modelValue")).toBeUndefined();
    wrapper.unmount();
  });

  it("honours disabled options", async () => {
    const wrapper = mount(TdLanguageSwitcher, {
      props: {
        modelValue: "de",
        languages: [
          { value: "de", label: "Deutsch", short: "DE" },
          { value: "en", label: "English", short: "EN", disabled: true },
        ],
      },
      attachTo: document.body,
    });
    await wrapper.find("button").trigger("click");
    await settle();

    const english = document.body.querySelectorAll('[role="menuitem"]')[1] as HTMLElement;
    expect(english.hasAttribute("disabled")).toBe(true);
    wrapper.unmount();
  });

  it("uses only semantic tokens", () => {
    expect(source).not.toMatch(/#[0-9a-f]{3,8}\b/i);
    expect(source).toContain("var(--td-color-primary)");
  });

  it("passes an axe accessibility check", async () => {
    const wrapper = mount(TdLanguageSwitcher, {
      props: { modelValue: "de" },
      attachTo: document.body,
    });
    await expectNoA11yViolations(wrapper.element);
    wrapper.unmount();
  });
});
```

### Verification

```bash
pnpm --filter @topdata-software-gmbh/ui-kit-web-vue3 test -- TdLanguageSwitcher
pnpm --filter @topdata-software-gmbh/ui-kit-web-vue3 typecheck
```

---

## 8. Phase 3 — Kit: playground, spec, registry, and README

**Goal:** make the new primitives discoverable and testable in the playground, and
document the API.

### Playground

```vue
<!-- [MODIFY] playground/src/App.vue — script additions -->
<script setup lang="ts">
import {
  TdLanguageSwitcher,
  useLanguage,
  type TdLanguageOption,
} from "@topdata-software-gmbh/ui-kit-web-vue3";

const { language, setLanguage } = useLanguage({
  defaultLanguage: "en",
  supported: ["en", "de"],
});

const languages: TdLanguageOption[] = [
  { value: "en", label: "English", short: "EN" },
  { value: "de", label: "Deutsch", short: "DE" },
];
</script>
```

```vue
<!-- [MODIFY] playground/src/App.vue — header actions -->
<div class="row">
  <TdLanguageSwitcher
    :model-value="language"
    :languages="languages"
    aria-label="Change language"
    @update:model-value="setLanguage"
  />
  <TdButton variant="secondary" @click="toggle">
    {{ mode === "dark" ? "Switch to light" : "Switch to dark" }}
  </TdButton>
</div>
```

Also add a short section near the action components documenting that the switcher
is presentation-only and that the consuming app supplies the language state.

### Spec

```md
<!-- [NEW FILE] _ai/specs/SPEC__261031-1__td-language-switcher.md -->
---
filename: "_ai/specs/SPEC__261031-1__td-language-switcher.md"
title: "TdLanguageSwitcher and useLanguage"
specId: 261031-1
status: Accepted
date: 2026-10-31
priority: high
estimatedComplexity: simple
tags: [design-system, ui, i18n, accessibility]
documentType: SPEC
---

# TdLanguageSwitcher and useLanguage

## Requirement: Language preference

`useLanguage` SHALL persist a supported language code, apply it to
`document.documentElement.lang`, fall back to a caller-provided default, and reject
unsupported values.

### Scenario: Persist and apply

- **WHEN** `setLanguage("de")` is called with `"de"` supported
- **THEN** the code is stored under `topdata.language.v1`, the reactive value
  changes, and `<html lang>` becomes `"de"`

### Scenario: Unsupported value

- **WHEN** a stored or requested language is not in `supported`
- **THEN** the default language remains active and nothing is persisted

## Requirement: Switcher

`TdLanguageSwitcher` SHALL render the active language code, open a menu of language
endonyms, mark the active option, and emit `update:modelValue` with the selected
value.

### Scenario: Selection

- **WHEN** the user selects a different language
- **THEN** `update:modelValue` emits that value and the menu closes

### Scenario: Accessibility

- **WHEN** the switcher renders
- **THEN** the trigger has an accessible name that includes the active language and
  the component passes an axe WCAG A/AA check
```

### Index and docs

```md
<!-- [MODIFY] _ai/specs/index.md — append under 2026 -->
* [SPEC__261031-1__td-language-switcher.md](SPEC__261031-1__td-language-switcher.md)
```

````md
<!-- [MODIFY] packages/ui-kit-web-vue3/README.md — component table -->
| `TdLanguageSwitcher` | Compact language menu with active-code trigger and endonym options |
| `useLanguage` | Language preference, persistence, and `<html lang>` sync |

<!-- [MODIFY] packages/ui-kit-web-vue3/README.md — traceability table -->
| `TdLanguageSwitcher`, `useLanguage` | `_ai/specs/SPEC__261031-1__td-language-switcher.md` | `src/components/TdLanguageSwitcher.test.ts`, `src/composables/useLanguage.test.ts` |

<!-- [MODIFY] packages/ui-kit-web-vue3/README.md — after Theme -->
### Language

```ts
import { useLanguage, TdLanguageSwitcher } from "@topdata-software-gmbh/ui-kit-web-vue3";

const { language, setLanguage } = useLanguage({
  defaultLanguage: "de",
  supported: ["de", "en"],
});
```

`useLanguage` writes `topdata.language.v1` and keeps `<html lang>` in sync. The kit
ships no dictionary; the consuming app owns its copy.
````

```yaml
# [MODIFY] packages/ui-component-registry.yaml — metadata only
updated_at: "2026-10-31"
```

### Verification

```bash
cd /topdata/topdata-design-system
pnpm --filter @topdata-software-gmbh/ui-kit-web-vue3 build
pnpm typecheck
pnpm test
```

---

## 9. Phase 4 — App: i18n runtime and English catalogue

**Goal:** provide the translation abstraction and the complete English key set.

### Runtime

```ts
// [NEW FILE] src/lib/i18n/index.ts
import { computed, type Ref } from 'vue';
import { useLanguage } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { en } from './messages.en';
import { de } from './messages.de';

export type Language = 'en' | 'de';
export const LANGUAGES: readonly Language[] = ['de', 'en'];

export type MessageParams = Record<string, string | number>;
export type MessageValue = string | ((params: MessageParams) => string);
export type MessageKey = keyof typeof en;
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
```

> **Import-cycle note:** `messages.de.ts` imports `type MessageKey` from
> `messages.en.ts`; `index.ts` imports both values. That is a type-only edge, so no
> runtime cycle.

### English catalogue

```ts
// [NEW FILE] src/lib/i18n/messages.en.ts
/**
 * Single source of truth for English copy.
 * `messages.de.ts` is typed against `MessageKey`, so adding a key here forces a
 * matching German entry at compile time.
 */
export const en = {
  // Common
  'common.project.one': 'project',
  'common.project.other': 'projects',
  'common.group.one': 'group',
  'common.group.other': 'groups',
  'common.file.one': 'file',
  'common.file.other': 'files',
  'common.shop.one': 'shop',
  'common.shop.other': 'shops',
  'common.update.one': 'update',
  'common.update.other': 'updates',
  'common.andMore': '…and {count} more.',
  'common.unknownError': 'Unknown error',
  'common.of': '{current} of {total}',
  'common.projectsSelected': '{count} of {total} {projectWord} selected',
  'common.cancel': 'Cancel',
  'common.continue': 'Continue',
  'common.back': 'Back',
  'common.delete': 'Delete',

  // App shell
  'app.liveUpdatesUnavailable.title': 'Live updates unavailable',

  // Sidebar
  'sidebar.allProjects': 'All projects',
  'sidebar.groups': 'Groups',
  'sidebar.newGroup': 'New group',
  'sidebar.editGroup': 'Edit group',
  'sidebar.createGroupHint': 'Create a group to act on several projects at once.',
  'sidebar.folders': 'Folders',
  'sidebar.addFolder': 'Add folder',
  'sidebar.addFolderHint': 'Add the folder where your projects live.',
  'sidebar.stopLooking': 'Stop looking in this folder',
  'sidebar.startLooking': 'Look in this folder again',
  'sidebar.removeFolder': 'Remove folder',
  'sidebar.looking': 'Looking…',
  'sidebar.lookAgain': 'Look again',
  'sidebar.switchToLight': 'Switch to light mode',
  'sidebar.switchToDark': 'Switch to dark mode',
  'sidebar.moreOptions': 'More options',
  'sidebar.runSetupAgain': 'Run setup again',
  'sidebar.howThisWorks': 'How this works',
  'sidebar.changeLanguage': 'Change language',
  'sidebar.confirmRemoveFolder':
    'Stop looking for projects in "{path}"? Your projects and files stay exactly where they are.',

  // Toolbar
  'toolbar.projectGroup': 'Project group',
  'toolbar.noChangesWaiting': 'Projects with no changes waiting',
  'toolbar.upToDateCount': '{count} up to date',
  'toolbar.chooseVersionHint': 'Choose which version of the project you want to work on',
  'toolbar.loadingBranches': 'Loading branches',
  'toolbar.branch': 'Branch',
  'toolbar.workingOnBranch': 'Working on branch',
  'toolbar.noProjectsYet': 'No projects yet',
  'toolbar.chooseBranch': 'Choose a branch…',
  'toolbar.saveToGit': 'Save to Git',
  'toolbar.saveHint': 'Create a checkpoint of your changed files in Git',
  'toolbar.getLatest': 'Get latest',
  'toolbar.getLatestHint': 'Download new work from the server and update your project folders',
  'toolbar.sendChanges': 'Send changes',
  'toolbar.sendHint': 'Upload your saved checkpoints to the server',
  'toolbar.moreGitActions': 'More Git actions',
  'toolbar.checkForUpdates': 'Check for updates',
  'toolbar.checkForUpdatesHint': 'Looks for new work without changing your files.',
  'toolbar.setAside': 'Set aside changes',
  'toolbar.setAsideHint': 'Temporarily store unfinished work in all projects.',
  'toolbar.bringBack': 'Bring back set-aside changes',
  'toolbar.bringBackHint': 'Restore the last set of changes in all projects.',
  'toolbar.lookForProjects': 'Look for new projects',
  'toolbar.lookForProjectsHint': 'Check your folders for projects again.',
  'toolbar.whatGitDid': 'What Git did',
  'toolbar.whatGitDidHint': 'See the exact commands behind the last actions.',

  // Status
  'status.folderNotFound': 'Folder not found',
  'status.needsAttention': 'Needs attention',
  'status.changesNotSaved': 'Changes not saved',
  'status.updatesAvailable': '{count} {updateWord} available',
  'status.toSend': '{count} to send',
  'status.localOnly': 'On this computer only',
  'status.upToDate': 'Up to date',
  'status.mergeInProgress': 'Merge in progress',
  'status.rebaseInProgress': 'Rebase in progress',
  'status.cherryPickInProgress': 'Cherry-pick in progress',
  'status.operationInProgress': 'Git operation in progress',
  'status.notOnBranch': 'Not on a branch',
  'status.unknownBranch': 'Unknown',

  // Tables
  'table.project': 'Project',
  'table.status': 'Status',
  'table.lastChange': 'Last change',
  'table.actions': 'Actions',
  'table.notOnBranch': 'not on a branch',
  'table.branchHint': 'The branch (version) you are working on',
  'table.noCheckpoints': 'No checkpoints saved yet',
  'table.reviewConflictsHint': 'Review the changed files (resolve conflicts before saving)',
  'table.saveHint': 'Save changes in this project to Git',
  'table.review': 'Review',
  'table.save': 'Save',
  'table.getLatestFor': 'Get latest for {name}',
  'table.getLatestForHint': 'Get latest for this project',
  'table.sendFor': 'Send changes for {name}',
  'table.sendForHint': 'Send changes for this project',
  'table.refreshFor': 'Refresh {name}',
  'table.refreshHint': 'Refresh this project',
  'table.searchPlaceholder': 'Search projects…',
  'table.searchLabel': 'Search projects',

  // All repos / filters
  'allRepos.title': 'All projects',
  'allRepos.filter': 'Filter',
  'allRepos.searchLabel': 'Search all projects',
  'allRepos.clearFilters': 'Clear filters',
  'allRepos.noMatch': 'No projects match',
  'allRepos.noMatchHint': 'Try a different search or filter.',
  'allRepos.looking': 'Looking for projects…',
  'allRepos.checking': 'Checking {folders}. Large folders can take a minute.',
  'allRepos.yourFolders': 'your folders',
  'allRepos.noProjectsFound': 'No projects found',
  'allRepos.lookedIn': 'We looked in {folders} but did not find a Git project.',
  'allRepos.thatFolder': 'that folder',
  'allRepos.chooseAnother': 'Choose another folder',
  'allRepos.lookAgain': 'Look again',
  'allRepos.findProjects': 'Find your projects',
  'allRepos.findProjectsHint':
    'Choose the folder where your projects live. We will find the Git projects inside.',
  'allRepos.chooseFolder': 'Choose folder',
  'filters.group': 'Group',
  'filters.type': 'Type',
  'filters.status': 'Status',
  'filters.statusLabel': 'Filter by status',
  'filters.anyGroup': 'Any group',
  'filters.ungrouped': 'Ungrouped',
  'filters.anyType': 'Any type',
  'filters.theme': 'Theme',
  'filters.plugin': 'Plugin',
  'filters.other': 'Other',
  'filters.needsAttention': 'Needs attention',
  'filters.changesNotSaved': 'Changes not saved',
  'filters.updatesAvailable': 'Updates available',
  'filters.toSend': 'To send',
  'filters.upToDate': 'Up to date',
  'filters.localOnly': 'On this computer only',

  // Empty states
  'empty.findProjects.title': "Let's find your projects",
  'empty.findProjects.text':
    'Choose the folder where your themes and projects live. We will look inside it and add every Git project we find.',
  'empty.noProjectsInGroup.title': 'No projects in this group',
  'empty.noProjectsInGroup.text':
    'Add projects to {group} to use the buttons above for the whole group.',
  'empty.editGroup': 'Edit group',
  'empty.noGitProjects.title': 'No Git projects found',
  'empty.noGitProjects.text':
    'We looked inside your folders but did not find a Git project. Check that the folder is correct, or add another folder.',
  'empty.noMatch.title': 'No matching projects',
  'empty.noMatch.text': 'Try a different search, or clear the filter.',
  'empty.clearSearch': 'Clear search',
  'empty.addFolder': 'Add folder',
  'empty.howThisWorks': 'How this works',
  'empty.lookAgain': 'Look again',

  // Help
  'help.title': 'How this works',
  'help.description': 'A short guide to working with Git without the jargon.',
  'help.step1.title': 'Add your project folder once',
  'help.step1.text':
    'Choose the folder where your themes and projects live. The app finds the Git projects inside it.',
  'help.step2.title': 'Edit your files like you normally do',
  'help.step2.text':
    'Use your editor. The app watches each project and shows when something changed.',
  'help.step3.title': 'Save to Git when you reach a checkpoint',
  'help.step3.text':
    'A checkpoint is a commit: a snapshot of your work that you can return to later.',
  'help.step4.title': 'Get latest, then send changes',
  'help.step4.text':
    'Get latest before you start working. Send your saved changes when you are finished.',
  'help.wordsTitle': 'Git words in plain English',
  'help.word.save.name': 'Save to Git',
  'help.word.save.text':
    'Creates a checkpoint (commit) of your changes in that project. Your files stay in the same folder.',
  'help.word.pull.name': 'Get latest',
  'help.word.pull.text':
    'Downloads new work from the server and updates your project folder. Local changes are set aside and put back afterwards.',
  'help.word.push.name': 'Send changes',
  'help.word.push.text': 'Uploads your saved checkpoints so other people can see them.',
  'help.word.branch.name': 'Branch',
  'help.word.branch.text':
    'A separate line of work. Switching branches changes the files in your project folder to that version.',
  'help.word.stash.name': 'Set aside changes',
  'help.word.stash.text':
    'Temporarily stores unfinished changes (a stash) so the folder is clean. Bring them back when you need them.',
  'help.warningTitle': 'If something goes wrong',
  'help.warningText':
    'Read the message at the bottom of the screen. It always explains the next step in plain language. Your work is not deleted by the buttons in this app.',
  'help.gotIt': 'Got it',

  // Group editor
  'group.newTitle': 'New project group',
  'group.editTitle': 'Edit project group',
  'group.description': 'Name the group and choose the projects it contains.',
  'group.nameLabel': 'Group name',
  'group.namePlaceholder': 'Theme projects',
  'group.nameRequired': 'Give the group a name.',
  'group.color': 'Color',
  'group.useColor': 'Use {color} for this group',
  'group.projects': 'Projects',
  'group.selectAll': 'Select all',
  'group.clearAll': 'Clear all',
  'group.noProjects':
    'No projects yet. Add the folder where your projects live, then look for projects.',
  'group.selectedCount': '{count} of {total} {projectWord} selected',
  'group.advanced': 'Advanced settings',
  'group.autoStashLabel': 'Keep unsaved changes when switching branches',
  'group.autoStashDescription': 'Changes are set aside and put back automatically.',
  'group.syncBranchingLabel': 'Show the branch chooser for the group',
  'group.syncBranchingDescription':
    'Lets you switch all projects in this group together.',
  'group.baseBranchLabel': 'Starting branch for new work',
  'group.delete': 'Delete group',
  'group.deleteConfirm': 'Delete group "{name}"? Your projects and files are not touched.',
  'group.cancel': 'Cancel',
  'group.saveChanges': 'Save changes',
  'group.create': 'Create group',

  // Scan path modal
  'scan.title': 'Add a folder',
  'scan.pathLabel': 'Where are your projects?',
  'scan.pathHint':
    'For example ~/devel or /home/you/projects. We skip folders like node_modules and build output.',
  'scan.moreOptions': 'More options',
  'scan.depthLabel': 'How deep to look',
  'scan.depthHint': 'Nested projects can sit a few folders deep.',
  'scan.cancel': 'Cancel',
  'scan.find': 'Find my projects',

  // Save changes modal
  'save.title': 'Save to Git',
  'save.description': 'Create a checkpoint of your work. Nothing is deleted or moved.',
  'save.reading': 'Reading your changes',
  'save.readingEllipsis': 'Reading your changes…',
  'save.workingOn': 'Working on {branch}',
  'save.changed': '{count} changed',
  'save.messageLabel': 'What did you change?',
  'save.messageHint':
    'Write a short note for your future self. It will appear in the project history.',
  'save.messagePlaceholder':
    'For example: Updated the header layout and product card colors',
  'save.total': '{files} {fileWord} across {projects} {projectWord}',
  'save.cancel': 'Cancel',
  'save.submit': 'Save to Git',
  'save.couldNotRead': 'Could not read the changes in: {names}.',
  'save.savedTitle': 'Saved to Git',
  'save.savedBody': '{count} {projectWord} updated.',
  'save.status.new': 'New file',
  'save.status.deleted': 'Deleted',
  'save.status.renamed': 'Renamed',
  'save.status.conflict': 'Conflict',
  'save.status.changed': 'Changed',

  // Git log drawer (chrome only; backend transcript stays English)
  'gitLog.region': 'What Git did',
  'gitLog.title': 'What Git did',
  'gitLog.close': 'Close',
  'gitLog.empty':
    'Nothing yet. Save, get the latest, or send changes and it will show up here.',
  'gitLog.shops': '{count} {shopWord}',
  'gitLog.hideCommands': 'Hide commands',
  'gitLog.showCommands': 'Git commands',
  'gitLog.notNeeded': 'not needed this time',

  // Onboarding
  'onboarding.step.find': 'Find projects',
  'onboarding.step.choose': 'Choose projects',
  'onboarding.step.groups': 'Confirm groups',
  'onboarding.step.ready': 'Ready',
  'onboarding.stepCounter': 'Step {step} of 4 · {label}',
  'onboarding.title': "Let's set up your workspace",
  'onboarding.close': 'Close setup and resume later',
  'onboarding.progressLabel': 'Setup progress',
  'onboarding.find.title': 'Where do your projects live?',
  'onboarding.find.text':
    'Choose the folder that holds your themes and plugins. We will find the Git projects inside.',
  'onboarding.find.pathLabel': 'Project folder',
  'onboarding.find.pathHint': 'For example ~/devel or /home/you/projects.',
  'onboarding.find.moreOptions': 'More options',
  'onboarding.find.ownersLabel': 'Remote owner names (optional)',
  'onboarding.find.ownersHint':
    'Used only to pre-select projects you own. This stays on this computer.',
  'onboarding.find.depthLabel': 'How deep to look',
  'onboarding.find.depthHint': 'The default of 4 finds most setups.',
  'onboarding.find.groupingLabel': 'How to suggest groups',
  'onboarding.find.groupingHint': 'You can rename or skip groups later.',
  'onboarding.find.groupingTopLevel': 'By top-level folder',
  'onboarding.find.groupingSingle': 'One group for everything',
  'onboarding.find.scanning': 'Looking for projects. Large folders can take a minute…',
  'onboarding.find.submit': 'Find my projects',
  'onboarding.choose.title': 'Choose the projects to work with',
  'onboarding.choose.text':
    'Everything we found is listed here. Projects you own are already selected; adjust the list as you like.',
  'onboarding.choose.searchPlaceholder': 'Search name, path, or branch…',
  'onboarding.choose.searchLabel': 'Search projects',
  'onboarding.choose.selectVisible': 'Select all visible',
  'onboarding.choose.clearSelection': 'Clear selection',
  'onboarding.choose.noMatch': 'No projects match',
  'onboarding.choose.noMatchHint': 'Try a different search term.',
  'onboarding.choose.clearSearch': 'Clear search',
  'onboarding.choose.noProjects': 'No Git projects found',
  'onboarding.choose.noProjectsHint': 'Check the folder, or go back and choose another one.',
  'onboarding.groups.title': 'Confirm your groups',
  'onboarding.groups.text':
    'We suggested these from your folders. Rename them or leave one out; you can fine-tune membership later.',
  'onboarding.groups.accepted': '{count} {projectWord} accepted',
  'onboarding.groups.include': 'Include {name}',
  'onboarding.groups.nameLabel': 'Name for group {index}',
  'onboarding.groups.more': ' and {count} more',
  'onboarding.groups.empty':
    'No groups to suggest. Go back and select at least one project.',
  'onboarding.groups.finish': 'Finish setup',
  'onboarding.groups.errorKeepOne': 'Keep at least one group to continue.',
  'onboarding.groups.errorNameAll': 'Give every included group a name.',
  'onboarding.groups.errorUnique': 'Give each group a different name.',
  'onboarding.ready.title': "You're ready to go",
  'onboarding.ready.summary':
    '{projects} {projectWord} accepted and {groups} {groupWord} ready.',
  'onboarding.ready.saveTitle': 'Save to Git',
  'onboarding.ready.saveText':
    'Saves a checkpoint of your changed files. Your files stay where they are.',
  'onboarding.ready.pullTitle': 'Get latest',
  'onboarding.ready.pullText': 'Brings new work from the server into your project folders.',
  'onboarding.ready.pushTitle': 'Send changes',
  'onboarding.ready.pushText': 'Uploads your saved checkpoints so others can see them.',
  'onboarding.ready.openTitle': 'Open a workspace',
  'onboarding.ready.openGroup': 'Open {name}',
  'onboarding.ready.openAll': 'Open All projects',
  'onboarding.projectsGroup': 'Projects',

  // Actions and toasts
  'action.fetch': 'Check for updates',
  'action.pull': 'Get latest',
  'action.push': 'Send changes',
  'action.stash': 'Set aside changes',
  'action.stashPop': 'Bring back set-aside changes',
  'toast.fetchDone': 'Update check complete',
  'toast.fetchFailed': 'Update check failed',
  'toast.fetchFailure': 'Update check',
  'toast.pullDone': 'Latest changes downloaded',
  'toast.pullFailed': 'Could not get the latest changes',
  'toast.pullFailure': 'Get latest',
  'toast.pushDone': 'Changes sent',
  'toast.pushFailed': 'Could not send changes',
  'toast.pushFailure': 'Send changes',
  'toast.stashDone': 'Changes set aside',
  'toast.stashFailed': 'Could not set changes aside',
  'toast.stashFailure': 'Set aside changes',
  'toast.stashPopDone': 'Set-aside changes restored',
  'toast.stashPopFailed': 'Could not restore set-aside changes',
  'toast.stashPopFailure': 'Bring back changes',
  'toast.branchSwitched': 'Branch switched',
  'toast.branchSwitchFailed': 'Could not switch branch',
  'toast.branchSwitchFailure': 'Branch switch',
  'toast.complete': '{title} complete',
  'toast.resultCount': '{count} {projectWord} updated.',
  'toast.failedCount': '{title}: {failed} of {total} failed',
  'toast.groupSaved': 'Group saved',
  'toast.groupDeleted': 'Group deleted',
  'toast.scanPathAdded': 'Project folder added',
  'toast.scanPathRemoved': 'Project folder removed',
  'toast.setupComplete': 'Setup complete',
  'toast.groupsReady': '{count} {groupWord} ready.',
  'toast.switchTo': 'Switch to {branch}',
  'toast.switchToFailed': 'Switch to {branch} failed',
  'toast.loadConfigFailed': 'Could not load configuration',
  'toast.scanFailed': 'Could not look for projects',
  'toast.branchesFailed': 'Could not list branches',
  'toast.saveGroupFailed': 'Could not save group',
  'toast.deleteGroupFailed': 'Could not delete group',
  'toast.addScanPathFailed': 'Could not add project folder',
  'toast.removeScanPathFailed': 'Could not remove project folder',
  'toast.saveSetupFailed': 'Could not save setup progress',
  'toast.finishSetupFailed': 'Could not finish setup',

  // Store insights
  'insight.missing.one': '1 project cannot be found',
  'insight.missing.other': '{count} projects cannot be found',
  'insight.missing.text':
    'The folder may have been moved or renamed. Update the project folder or remove it from the scan list.',
  'insight.conflicts.one': '1 project needs attention',
  'insight.conflicts.other': '{count} projects need attention',
  'insight.conflicts.text':
    'Some files have conflicts. Open the project in your editor, fix the marked files, then click "Save to Git".',
  'insight.conflicts.action': 'Review changes',
  'insight.busy.title': 'A Git operation is still in progress',
  'insight.busy.text':
    'Finish or cancel the merge, rebase, or cherry-pick in your editor before using the buttons here.',
  'insight.detached.title': 'You are not on a branch',
  'insight.detached.text':
    'Switch to a branch below before saving changes. Otherwise new work can be hard to find later.',
  'insight.dirty.one': 'You have changes that are not saved to Git',
  'insight.dirty.other': '{count} projects have changes that are not saved to Git',
  'insight.dirty.text':
    'Save them to Git to create a checkpoint. Your files stay where they are, and you can keep working afterwards.',
  'insight.dirty.actionOne': 'Save to Git',
  'insight.dirty.actionOther': 'Save all to Git',
  'insight.behind.one': 'An update is available',
  'insight.behind.other': 'Updates are available',
  'insight.behind.text.one':
    '1 project can receive new work from the server. Get the latest changes before you start editing.',
  'insight.behind.text.other':
    '{count} projects can receive new work from the server. Get the latest changes before you start editing.',
  'insight.ahead.title': 'You have work ready to send',
  'insight.ahead.text.one':
    '1 project has saved changes that are not on the server yet. Send them when you are ready.',
  'insight.ahead.text.other':
    '{count} projects have saved changes that are not on the server yet. Send them when you are ready.',
  'insight.localOnly.title': 'Your work is saved on this computer',
  'insight.localOnly.text':
    'This project is not connected to a server, so there is nothing to send or receive.',
  'insight.upToDate.title': 'Everything is up to date',
  'insight.upToDate.text': 'No unsaved changes and no updates waiting.',

  // Validation
  'validation.chooseFolder': 'Choose the folder where your projects live.',
  'validation.scanFolderFailed': 'That folder could not be scanned. Check the path.',
  'validation.chooseProject': 'Choose at least one project to continue.',
  'validation.keepGroup': 'Keep at least one group to continue.',
  'validation.nameEveryGroup': 'Give every included group a name.',
  'validation.uniqueGroupNames': 'Give each group a different name.',
  'validation.confirmGroup': 'Confirm at least one group before finishing.',
} as const;
```

### Tests

```ts
// [NEW FILE] src/lib/__tests__/i18n.test.ts
import { beforeEach, describe, expect, it } from 'vitest';
import { language, setLanguage, t, tPlural } from '../i18n';
import { en } from '../i18n/messages.en';
import { de } from '../i18n/messages.de';

beforeEach(() => {
  setLanguage('de');
});

describe('catalogue', () => {
  it('has the same keys in English and German', () => {
    expect(Object.keys(de).sort()).toEqual(Object.keys(en).sort());
  });
});

describe('t', () => {
  it('translates a static key in the active language', () => {
    expect(t('toolbar.saveToGit')).toBe('In Git speichern');
    setLanguage('en');
    expect(t('toolbar.saveToGit')).toBe('Save to Git');
  });

  it('interpolates parameters and leaves unknown placeholders intact', () => {
    setLanguage('de');
    expect(t('toolbar.upToDateCount', { count: 3 })).toBe('3 aktuell');
    expect(t('common.of', { current: 2 })).toBe('2 von {total}');
  });

  it('uses the plural base key by count', () => {
    setLanguage('de');
    expect(tPlural('common.project', 1)).toBe('Projekt');
    expect(tPlural('common.project', 2)).toBe('Projekte');
    expect(tPlural('common.project', 0)).toBe('Projekte');
  });

  it('reacts to language changes', () => {
    const before = t('sidebar.groups');
    setLanguage('en');
    const after = t('sidebar.groups');

    expect(before).not.toBe(after);
    expect(language.value).toBe('en');
  });
});
```

### Verification

Phases 4 and 5 are one compile unit: `index.ts` imports `messages.de.ts`, and the
i18n test asserts German strings. Do not run typecheck until both files exist.
After Phase 5:

```bash
pnpm typecheck
pnpm test -- i18n
```

---

## 10. Phase 5 — App: German catalogue (informal du)

**Goal:** add every German message with consistent terminology and a friendly
informal tone.

### Terminology

| English | German |
|---|---|
| Save to Git | In Git speichern |
| Get latest | Neueste holen |
| Send changes | Änderungen senden |
| Set aside changes | Änderungen beiseitelegen |
| Bring back set-aside changes | Beiseitegelegte Änderungen zurückholen |
| Check for updates | Nach Updates suchen |
| checkpoint (commit) | Speicherpunkt (Commit) |
| project / projects | Projekt / Projekte |
| group / groups | Gruppe / Gruppen |
| shop / shops | Shop / Shops |
| branch | Branch |
| folder | Ordner |
| changes not saved | Änderungen nicht gespeichert |
| needs attention | braucht Aufmerksamkeit |
| up to date | aktuell |

### File

```ts
// [NEW FILE] src/lib/i18n/messages.de.ts
import type { MessageKey } from './messages.en';

/**
 * German copy, informal "du". Typed against the English catalogue so a missing
 * or misspelled key fails the build.
 */
export const de: Record<MessageKey, string> = {
  'common.project.one': 'Projekt',
  'common.project.other': 'Projekte',
  'common.group.one': 'Gruppe',
  'common.group.other': 'Gruppen',
  'common.file.one': 'Datei',
  'common.file.other': 'Dateien',
  'common.shop.one': 'Shop',
  'common.shop.other': 'Shops',
  'common.update.one': 'Update',
  'common.update.other': 'Updates',
  'common.andMore': '…und {count} weitere.',
  'common.unknownError': 'Unbekannter Fehler',
  'common.of': '{current} von {total}',
  'common.projectsSelected': '{count} von {total} {projectWord} ausgewählt',
  'common.cancel': 'Abbrechen',
  'common.continue': 'Weiter',
  'common.back': 'Zurück',
  'common.delete': 'Löschen',

  'app.liveUpdatesUnavailable.title': 'Live-Aktualisierungen nicht verfügbar',

  'sidebar.allProjects': 'Alle Projekte',
  'sidebar.groups': 'Gruppen',
  'sidebar.newGroup': 'Neue Gruppe',
  'sidebar.editGroup': 'Gruppe bearbeiten',
  'sidebar.createGroupHint':
    'Erstelle eine Gruppe, um mehrere Projekte gleichzeitig zu bearbeiten.',
  'sidebar.folders': 'Ordner',
  'sidebar.addFolder': 'Ordner hinzufügen',
  'sidebar.addFolderHint': 'Füge den Ordner hinzu, in dem deine Projekte liegen.',
  'sidebar.stopLooking': 'In diesem Ordner nicht mehr suchen',
  'sidebar.startLooking': 'Wieder in diesem Ordner suchen',
  'sidebar.removeFolder': 'Ordner entfernen',
  'sidebar.looking': 'Suche läuft…',
  'sidebar.lookAgain': 'Erneut suchen',
  'sidebar.switchToLight': 'Zum hellen Modus wechseln',
  'sidebar.switchToDark': 'Zum dunklen Modus wechseln',
  'sidebar.moreOptions': 'Weitere Optionen',
  'sidebar.runSetupAgain': 'Einrichtung erneut starten',
  'sidebar.howThisWorks': "So funktioniert's",
  'sidebar.changeLanguage': 'Sprache ändern',
  'sidebar.confirmRemoveFolder':
    'Nicht mehr nach Projekten in „{path}“ suchen? Deine Projekte und Dateien bleiben genau, wo sie sind.',

  'toolbar.projectGroup': 'Projektgruppe',
  'toolbar.noChangesWaiting': 'Projekte ohne ausstehende Änderungen',
  'toolbar.upToDateCount': '{count} aktuell',
  'toolbar.chooseVersionHint':
    'Wähle, an welcher Version des Projekts du arbeiten möchtest',
  'toolbar.loadingBranches': 'Branches werden geladen',
  'toolbar.branch': 'Branch',
  'toolbar.workingOnBranch': 'Du arbeitest an Branch',
  'toolbar.noProjectsYet': 'Noch keine Projekte',
  'toolbar.chooseBranch': 'Branch auswählen…',
  'toolbar.saveToGit': 'In Git speichern',
  'toolbar.saveHint': 'Erstelle einen Speicherpunkt deiner geänderten Dateien in Git',
  'toolbar.getLatest': 'Neueste holen',
  'toolbar.getLatestHint':
    'Lade neue Arbeit vom Server und aktualisiere deine Projektordner',
  'toolbar.sendChanges': 'Änderungen senden',
  'toolbar.sendHint': 'Lade deine gespeicherten Speicherpunkte zum Server hoch',
  'toolbar.moreGitActions': 'Weitere Git-Aktionen',
  'toolbar.checkForUpdates': 'Nach Updates suchen',
  'toolbar.checkForUpdatesHint':
    'Sucht nach neuer Arbeit, ohne deine Dateien zu verändern.',
  'toolbar.setAside': 'Änderungen beiseitelegen',
  'toolbar.setAsideHint':
    'Speichert unfertige Arbeit in allen Projekten vorübergehend.',
  'toolbar.bringBack': 'Beiseitegelegte Änderungen zurückholen',
  'toolbar.bringBackHint':
    'Holt die zuletzt beiseitegelegten Änderungen in allen Projekten zurück.',
  'toolbar.lookForProjects': 'Nach neuen Projekten suchen',
  'toolbar.lookForProjectsHint': 'Durchsuche deine Ordner erneut nach Projekten.',
  'toolbar.whatGitDid': 'Was Git gemacht hat',
  'toolbar.whatGitDidHint': 'Sieh dir die genauen Befehle der letzten Aktionen an.',

  'status.folderNotFound': 'Ordner nicht gefunden',
  'status.needsAttention': 'Braucht Aufmerksamkeit',
  'status.changesNotSaved': 'Änderungen nicht gespeichert',
  'status.updatesAvailable': '{count} {updateWord} verfügbar',
  'status.toSend': '{count} zu senden',
  'status.localOnly': 'Nur auf diesem Computer',
  'status.upToDate': 'Aktuell',
  'status.mergeInProgress': 'Merge läuft',
  'status.rebaseInProgress': 'Rebase läuft',
  'status.cherryPickInProgress': 'Cherry-Pick läuft',
  'status.operationInProgress': 'Git-Operation läuft',
  'status.notOnBranch': 'Auf keinem Branch',
  'status.unknownBranch': 'Unbekannt',

  'table.project': 'Projekt',
  'table.status': 'Status',
  'table.lastChange': 'Letzte Änderung',
  'table.actions': 'Aktionen',
  'table.notOnBranch': 'auf keinem Branch',
  'table.branchHint': 'Der Branch (die Version), an dem du arbeitest',
  'table.noCheckpoints': 'Noch keine Speicherpunkte',
  'table.reviewConflictsHint':
    'Prüfe die geänderten Dateien (löse Konflikte vor dem Speichern)',
  'table.saveHint': 'Speichere die Änderungen in diesem Projekt in Git',
  'table.review': 'Prüfen',
  'table.save': 'Speichern',
  'table.getLatestFor': 'Neueste für {name} holen',
  'table.getLatestForHint': 'Neueste für dieses Projekt holen',
  'table.sendFor': 'Änderungen für {name} senden',
  'table.sendForHint': 'Änderungen für dieses Projekt senden',
  'table.refreshFor': '{name} aktualisieren',
  'table.refreshHint': 'Dieses Projekt aktualisieren',
  'table.searchPlaceholder': 'Projekte suchen…',
  'table.searchLabel': 'Projekte suchen',

  'allRepos.title': 'Alle Projekte',
  'allRepos.filter': 'Filter',
  'allRepos.searchLabel': 'Alle Projekte durchsuchen',
  'allRepos.clearFilters': 'Filter zurücksetzen',
  'allRepos.noMatch': 'Keine passenden Projekte',
  'allRepos.noMatchHint': 'Versuche eine andere Suche oder einen anderen Filter.',
  'allRepos.looking': 'Projekte werden gesucht…',
  'allRepos.checking': 'Prüfe {folders}. Große Ordner können eine Minute dauern.',
  'allRepos.yourFolders': 'deine Ordner',
  'allRepos.noProjectsFound': 'Keine Projekte gefunden',
  'allRepos.lookedIn':
    'Wir haben in {folders} nachgesehen, aber kein Git-Projekt gefunden.',
  'allRepos.thatFolder': 'diesem Ordner',
  'allRepos.chooseAnother': 'Anderen Ordner wählen',
  'allRepos.lookAgain': 'Erneut suchen',
  'allRepos.findProjects': 'Finde deine Projekte',
  'allRepos.findProjectsHint':
    'Wähle den Ordner, in dem deine Projekte liegen. Wir finden die Git-Projekte darin.',
  'allRepos.chooseFolder': 'Ordner wählen',
  'filters.group': 'Gruppe',
  'filters.type': 'Typ',
  'filters.status': 'Status',
  'filters.statusLabel': 'Nach Status filtern',
  'filters.anyGroup': 'Alle Gruppen',
  'filters.ungrouped': 'Ohne Gruppe',
  'filters.anyType': 'Alle Typen',
  'filters.theme': 'Theme',
  'filters.plugin': 'Plugin',
  'filters.other': 'Andere',
  'filters.needsAttention': 'Braucht Aufmerksamkeit',
  'filters.changesNotSaved': 'Änderungen nicht gespeichert',
  'filters.updatesAvailable': 'Updates verfügbar',
  'filters.toSend': 'Zu senden',
  'filters.upToDate': 'Aktuell',
  'filters.localOnly': 'Nur auf diesem Computer',

  'empty.findProjects.title': 'Lass uns deine Projekte finden',
  'empty.findProjects.text':
    'Wähle den Ordner, in dem deine Themes und Projekte liegen. Wir schauen hinein und fügen jedes Git-Projekt hinzu, das wir finden.',
  'empty.noProjectsInGroup.title': 'Keine Projekte in dieser Gruppe',
  'empty.noProjectsInGroup.text':
    'Füge Projekte zu {group} hinzu, um die Schaltflächen oben für die ganze Gruppe zu nutzen.',
  'empty.editGroup': 'Gruppe bearbeiten',
  'empty.noGitProjects.title': 'Keine Git-Projekte gefunden',
  'empty.noGitProjects.text':
    'Wir haben in deinen Ordnern nachgesehen, aber kein Git-Projekt gefunden. Prüfe, ob der Ordner stimmt, oder füge einen anderen Ordner hinzu.',
  'empty.noMatch.title': 'Keine passenden Projekte',
  'empty.noMatch.text': 'Versuche eine andere Suche oder setze den Filter zurück.',
  'empty.clearSearch': 'Suche zurücksetzen',
  'empty.addFolder': 'Ordner hinzufügen',
  'empty.howThisWorks': "So funktioniert's",
  'empty.lookAgain': 'Erneut suchen',

  'help.title': "So funktioniert's",
  'help.description': 'Eine kurze Anleitung für die Arbeit mit Git ohne Fachchinesisch.',
  'help.step1.title': 'Füge deinen Projektordner einmal hinzu',
  'help.step1.text':
    'Wähle den Ordner, in dem deine Themes und Projekte liegen. Die App findet die Git-Projekte darin.',
  'help.step2.title': 'Bearbeite deine Dateien wie gewohnt',
  'help.step2.text':
    'Nutze deinen Editor. Die App beobachtet jedes Projekt und zeigt, wenn sich etwas geändert hat.',
  'help.step3.title': 'Speichere in Git, wenn du einen Speicherpunkt erreichst',
  'help.step3.text':
    'Ein Speicherpunkt (Commit) ist ein Schnappschuss deiner Arbeit, zu dem du später zurückkehren kannst.',
  'help.step4.title': 'Hol dir das Neueste und sende dann deine Änderungen',
  'help.step4.text':
    'Hol dir vor der Arbeit das Neueste. Sende deine gespeicherten Änderungen, wenn du fertig bist.',
  'help.wordsTitle': 'Git-Begriffe einfach erklärt',
  'help.word.save.name': 'In Git speichern',
  'help.word.save.text':
    'Erstellt einen Speicherpunkt (Commit) deiner Änderungen in diesem Projekt. Deine Dateien bleiben im selben Ordner.',
  'help.word.pull.name': 'Neueste holen',
  'help.word.pull.text':
    'Lädt neue Arbeit vom Server und aktualisiert deinen Projektordner. Lokale Änderungen werden beiseitegelegt und danach zurückgeholt.',
  'help.word.push.name': 'Änderungen senden',
  'help.word.push.text':
    'Lädt deine gespeicherten Speicherpunkte hoch, damit andere sie sehen.',
  'help.word.branch.name': 'Branch',
  'help.word.branch.text':
    'Ein separater Arbeitsstrang. Beim Wechseln des Branches werden die Dateien in deinem Projektordner auf diese Version gebracht.',
  'help.word.stash.name': 'Änderungen beiseitelegen',
  'help.word.stash.text':
    'Legt unfertige Änderungen vorübergehend beiseite (ein Stash), damit der Ordner sauber ist. Hol sie zurück, wenn du sie brauchst.',
  'help.warningTitle': 'Wenn etwas schiefgeht',
  'help.warningText':
    'Lies die Meldung am unteren Bildschirmrand. Sie erklärt immer den nächsten Schritt in einfacher Sprache. Deine Arbeit wird durch die Schaltflächen in dieser App nicht gelöscht.',
  'help.gotIt': 'Verstanden',

  'group.newTitle': 'Neue Projektgruppe',
  'group.editTitle': 'Projektgruppe bearbeiten',
  'group.description': 'Benenne die Gruppe und wähle die enthaltenen Projekte.',
  'group.nameLabel': 'Gruppenname',
  'group.namePlaceholder': 'Theme-Projekte',
  'group.nameRequired': 'Gib der Gruppe einen Namen.',
  'group.color': 'Farbe',
  'group.useColor': 'Farbe {color} für diese Gruppe verwenden',
  'group.projects': 'Projekte',
  'group.selectAll': 'Alle auswählen',
  'group.clearAll': 'Auswahl aufheben',
  'group.noProjects':
    'Noch keine Projekte. Füge den Ordner hinzu, in dem deine Projekte liegen, und suche dann nach Projekten.',
  'group.selectedCount': '{count} von {total} {projectWord} ausgewählt',
  'group.advanced': 'Erweiterte Einstellungen',
  'group.autoStashLabel': 'Ungespeicherte Änderungen beim Branch-Wechsel behalten',
  'group.autoStashDescription':
    'Änderungen werden automatisch beiseitegelegt und zurückgeholt.',
  'group.syncBranchingLabel': 'Branch-Auswahl für die Gruppe anzeigen',
  'group.syncBranchingDescription':
    'Damit kannst du alle Projekte dieser Gruppe gemeinsam umstellen.',
  'group.baseBranchLabel': 'Start-Branch für neue Arbeit',
  'group.delete': 'Gruppe löschen',
  'group.deleteConfirm':
    'Gruppe „{name}“ löschen? Deine Projekte und Dateien werden nicht angefasst.',
  'group.cancel': 'Abbrechen',
  'group.saveChanges': 'Änderungen speichern',
  'group.create': 'Gruppe erstellen',

  'scan.title': 'Ordner hinzufügen',
  'scan.pathLabel': 'Wo liegen deine Projekte?',
  'scan.pathHint':
    'Zum Beispiel ~/devel oder /home/du/projekte. Ordner wie node_modules und Build-Ausgaben überspringen wir.',
  'scan.moreOptions': 'Weitere Optionen',
  'scan.depthLabel': 'Wie tief suchen',
  'scan.depthHint': 'Verschachtelte Projekte können einige Ordnerebenen tiefer liegen.',
  'scan.cancel': 'Abbrechen',
  'scan.find': 'Meine Projekte finden',

  'save.title': 'In Git speichern',
  'save.description':
    'Erstelle einen Speicherpunkt deiner Arbeit. Nichts wird gelöscht oder verschoben.',
  'save.reading': 'Änderungen werden gelesen',
  'save.readingEllipsis': 'Änderungen werden gelesen…',
  'save.workingOn': 'Du arbeitest an {branch}',
  'save.changed': '{count} geändert',
  'save.messageLabel': 'Was hast du geändert?',
  'save.messageHint':
    'Schreib eine kurze Notiz für dein zukünftiges Ich. Sie erscheint in der Projekthistorie.',
  'save.messagePlaceholder':
    'Zum Beispiel: Header-Layout und Farben der Produktkarten aktualisiert',
  'save.total': '{files} {fileWord} in {projects} {projectWord}',
  'save.cancel': 'Abbrechen',
  'save.submit': 'In Git speichern',
  'save.couldNotRead': 'Die Änderungen konnten nicht gelesen werden in: {names}.',
  'save.savedTitle': 'In Git gespeichert',
  'save.savedBody': '{count} {projectWord} aktualisiert.',
  'save.status.new': 'Neue Datei',
  'save.status.deleted': 'Gelöscht',
  'save.status.renamed': 'Umbenannt',
  'save.status.conflict': 'Konflikt',
  'save.status.changed': 'Geändert',

  'gitLog.region': 'Was Git gemacht hat',
  'gitLog.title': 'Was Git gemacht hat',
  'gitLog.close': 'Schließen',
  'gitLog.empty':
    'Noch nichts. Speichere, hol dir das Neueste oder sende Änderungen, dann erscheint es hier.',
  'gitLog.shops': '{count} {shopWord}',
  'gitLog.hideCommands': 'Befehle ausblenden',
  'gitLog.showCommands': 'Git-Befehle',
  'gitLog.notNeeded': 'diesmal nicht nötig',

  'onboarding.step.find': 'Projekte finden',
  'onboarding.step.choose': 'Projekte auswählen',
  'onboarding.step.groups': 'Gruppen bestätigen',
  'onboarding.step.ready': 'Fertig',
  'onboarding.stepCounter': 'Schritt {step} von 4 · {label}',
  'onboarding.title': 'Lass uns deinen Arbeitsbereich einrichten',
  'onboarding.close': 'Einrichtung schließen und später fortsetzen',
  'onboarding.progressLabel': 'Einrichtungsfortschritt',
  'onboarding.find.title': 'Wo liegen deine Projekte?',
  'onboarding.find.text':
    'Wähle den Ordner mit deinen Themes und Plugins. Wir finden die Git-Projekte darin.',
  'onboarding.find.pathLabel': 'Projektordner',
  'onboarding.find.pathHint': 'Zum Beispiel ~/devel oder /home/du/projekte.',
  'onboarding.find.moreOptions': 'Weitere Optionen',
  'onboarding.find.ownersLabel': 'Namen der Remote-Inhaber (optional)',
  'onboarding.find.ownersHint':
    'Wird nur verwendet, um Projekte von dir vorauszuwählen. Das bleibt auf diesem Computer.',
  'onboarding.find.depthLabel': 'Wie tief suchen',
  'onboarding.find.depthHint': 'Der Standardwert 4 findet die meisten Setups.',
  'onboarding.find.groupingLabel': 'Wie Gruppen vorgeschlagen werden',
  'onboarding.find.groupingHint': 'Du kannst Gruppen später umbenennen oder überspringen.',
  'onboarding.find.groupingTopLevel': 'Nach oberster Ordnerebene',
  'onboarding.find.groupingSingle': 'Eine Gruppe für alles',
  'onboarding.find.scanning':
    'Projekte werden gesucht. Große Ordner können eine Minute dauern…',
  'onboarding.find.submit': 'Meine Projekte finden',
  'onboarding.choose.title': 'Wähle die Projekte aus, mit denen du arbeiten willst',
  'onboarding.choose.text':
    'Alles, was wir gefunden haben, steht hier. Projekte von dir sind bereits ausgewählt; passe die Liste nach Wunsch an.',
  'onboarding.choose.searchPlaceholder': 'Name, Pfad oder Branch suchen…',
  'onboarding.choose.searchLabel': 'Projekte suchen',
  'onboarding.choose.selectVisible': 'Alle sichtbaren auswählen',
  'onboarding.choose.clearSelection': 'Auswahl aufheben',
  'onboarding.choose.noMatch': 'Keine passenden Projekte',
  'onboarding.choose.noMatchHint': 'Versuch einen anderen Suchbegriff.',
  'onboarding.choose.clearSearch': 'Suche zurücksetzen',
  'onboarding.choose.noProjects': 'Keine Git-Projekte gefunden',
  'onboarding.choose.noProjectsHint':
    'Prüfe den Ordner oder geh zurück und wähle einen anderen.',
  'onboarding.groups.title': 'Bestätige deine Gruppen',
  'onboarding.groups.text':
    'Wir haben diese Gruppen aus deinen Ordnern vorgeschlagen. Benenne sie um oder lass eine weg; die Mitglieder kannst du später anpassen.',
  'onboarding.groups.accepted': '{count} {projectWord} übernommen',
  'onboarding.groups.include': '{name} einschließen',
  'onboarding.groups.nameLabel': 'Name für Gruppe {index}',
  'onboarding.groups.more': ' und {count} weitere',
  'onboarding.groups.empty':
    'Keine Gruppen zum Vorschlagen. Geh zurück und wähle mindestens ein Projekt aus.',
  'onboarding.groups.finish': 'Einrichtung abschließen',
  'onboarding.groups.errorKeepOne': 'Behalte mindestens eine Gruppe, um fortzufahren.',
  'onboarding.groups.errorNameAll': 'Gib jeder ausgewählten Gruppe einen Namen.',
  'onboarding.groups.errorUnique': 'Gib jeder Gruppe einen anderen Namen.',
  'onboarding.ready.title': 'Du kannst loslegen',
  'onboarding.ready.summary':
    '{projects} {projectWord} übernommen und {groups} {groupWord} bereit.',
  'onboarding.ready.saveTitle': 'In Git speichern',
  'onboarding.ready.saveText':
    'Speichert einen Speicherpunkt deiner geänderten Dateien. Deine Dateien bleiben, wo sie sind.',
  'onboarding.ready.pullTitle': 'Neueste holen',
  'onboarding.ready.pullText': 'Holt neue Arbeit vom Server in deine Projektordner.',
  'onboarding.ready.pushTitle': 'Änderungen senden',
  'onboarding.ready.pushText':
    'Lädt deine gespeicherten Speicherpunkte hoch, damit andere sie sehen.',
  'onboarding.ready.openTitle': 'Arbeitsbereich öffnen',
  'onboarding.ready.openGroup': '{name} öffnen',
  'onboarding.ready.openAll': 'Alle Projekte öffnen',
  'onboarding.projectsGroup': 'Projekte',

  'action.fetch': 'Nach Updates suchen',
  'action.pull': 'Neueste holen',
  'action.push': 'Änderungen senden',
  'action.stash': 'Änderungen beiseitelegen',
  'action.stashPop': 'Beiseitegelegte Änderungen zurückholen',
  'toast.fetchDone': 'Update-Prüfung abgeschlossen',
  'toast.fetchFailed': 'Update-Prüfung fehlgeschlagen',
  'toast.fetchFailure': 'Update-Prüfung',
  'toast.pullDone': 'Neueste Änderungen geholt',
  'toast.pullFailed': 'Neueste Änderungen konnten nicht geholt werden',
  'toast.pullFailure': 'Neueste holen',
  'toast.pushDone': 'Änderungen gesendet',
  'toast.pushFailed': 'Änderungen konnten nicht gesendet werden',
  'toast.pushFailure': 'Änderungen senden',
  'toast.stashDone': 'Änderungen beiseitegelegt',
  'toast.stashFailed': 'Änderungen konnten nicht beiseitegelegt werden',
  'toast.stashFailure': 'Änderungen beiseitelegen',
  'toast.stashPopDone': 'Änderungen zurückgeholt',
  'toast.stashPopFailed': 'Änderungen konnten nicht zurückgeholt werden',
  'toast.stashPopFailure': 'Änderungen zurückholen',
  'toast.branchSwitched': 'Branch gewechselt',
  'toast.branchSwitchFailed': 'Branch-Wechsel fehlgeschlagen',
  'toast.branchSwitchFailure': 'Branch-Wechsel',
  'toast.complete': '{title} abgeschlossen',
  'toast.resultCount': '{count} {projectWord} aktualisiert.',
  'toast.failedCount': '{title}: {failed} von {total} fehlgeschlagen',
  'toast.groupSaved': 'Gruppe gespeichert',
  'toast.groupDeleted': 'Gruppe gelöscht',
  'toast.scanPathAdded': 'Projektordner hinzugefügt',
  'toast.scanPathRemoved': 'Projektordner entfernt',
  'toast.setupComplete': 'Einrichtung abgeschlossen',
  'toast.groupsReady': '{count} {groupWord} bereit.',
  'toast.switchTo': 'Wechsel zu {branch}',
  'toast.switchToFailed': 'Wechsel zu {branch} fehlgeschlagen',
  'toast.loadConfigFailed': 'Konfiguration konnte nicht geladen werden',
  'toast.scanFailed': 'Projekte konnten nicht gesucht werden',
  'toast.branchesFailed': 'Branches konnten nicht geladen werden',
  'toast.saveGroupFailed': 'Gruppe konnte nicht gespeichert werden',
  'toast.deleteGroupFailed': 'Gruppe konnte nicht gelöscht werden',
  'toast.addScanPathFailed': 'Projektordner konnte nicht hinzugefügt werden',
  'toast.removeScanPathFailed': 'Projektordner konnte nicht entfernt werden',
  'toast.saveSetupFailed': 'Einrichtungsfortschritt konnte nicht gespeichert werden',
  'toast.finishSetupFailed': 'Einrichtung konnte nicht abgeschlossen werden',

  'insight.missing.one': '1 Projekt wurde nicht gefunden',
  'insight.missing.other': '{count} Projekte wurden nicht gefunden',
  'insight.missing.text':
    'Der Ordner wurde möglicherweise verschoben oder umbenannt. Aktualisiere den Projektordner oder entferne ihn aus der Suchliste.',
  'insight.conflicts.one': '1 Projekt braucht Aufmerksamkeit',
  'insight.conflicts.other': '{count} Projekte brauchen Aufmerksamkeit',
  'insight.conflicts.text':
    'Einige Dateien haben Konflikte. Öffne das Projekt in deinem Editor, behebe die markierten Dateien und klicke dann auf „In Git speichern“.',
  'insight.conflicts.action': 'Änderungen prüfen',
  'insight.busy.title': 'Eine Git-Operation läuft noch',
  'insight.busy.text':
    'Schließe den Merge, Rebase oder Cherry-Pick in deinem Editor ab oder brich ihn ab, bevor du die Schaltflächen hier verwendest.',
  'insight.detached.title': 'Du bist auf keinem Branch',
  'insight.detached.text':
    'Wechsle unten zu einem Branch, bevor du Änderungen speicherst. Sonst ist neue Arbeit später schwer zu finden.',
  'insight.dirty.one': 'Du hast Änderungen, die nicht in Git gespeichert sind',
  'insight.dirty.other':
    '{count} Projekte haben Änderungen, die nicht in Git gespeichert sind',
  'insight.dirty.text':
    'Speichere sie in Git, um einen Speicherpunkt zu erstellen. Deine Dateien bleiben, wo sie sind, und du kannst danach weiterarbeiten.',
  'insight.dirty.actionOne': 'In Git speichern',
  'insight.dirty.actionOther': 'Alle in Git speichern',
  'insight.behind.one': 'Ein Update ist verfügbar',
  'insight.behind.other': 'Updates sind verfügbar',
  'insight.behind.text.one':
    '1 Projekt kann neue Arbeit vom Server erhalten. Hol dir die neuesten Änderungen, bevor du mit der Bearbeitung beginnst.',
  'insight.behind.text.other':
    '{count} Projekte können neue Arbeit vom Server erhalten. Hol dir die neuesten Änderungen, bevor du mit der Bearbeitung beginnst.',
  'insight.ahead.title': 'Du hast Arbeit zum Senden bereit',
  'insight.ahead.text.one':
    '1 Projekt hat gespeicherte Änderungen, die noch nicht auf dem Server sind. Sende sie, wenn du bereit bist.',
  'insight.ahead.text.other':
    '{count} Projekte haben gespeicherte Änderungen, die noch nicht auf dem Server sind. Sende sie, wenn du bereit bist.',
  'insight.localOnly.title': 'Deine Arbeit ist auf diesem Computer gespeichert',
  'insight.localOnly.text':
    'Dieses Projekt ist nicht mit einem Server verbunden, deshalb gibt es nichts zu senden oder zu empfangen.',
  'insight.upToDate.title': 'Alles ist aktuell',
  'insight.upToDate.text': 'Keine ungespeicherten Änderungen und keine ausstehenden Updates.',

  'validation.chooseFolder': 'Wähle den Ordner, in dem deine Projekte liegen.',
  'validation.scanFolderFailed':
    'Dieser Ordner konnte nicht durchsucht werden. Prüfe den Pfad.',
  'validation.chooseProject': 'Wähle mindestens ein Projekt, um fortzufahren.',
  'validation.keepGroup': 'Behalte mindestens eine Gruppe, um fortzufahren.',
  'validation.nameEveryGroup': 'Gib jeder ausgewählten Gruppe einen Namen.',
  'validation.uniqueGroupNames': 'Gib jeder Gruppe einen anderen Namen.',
  'validation.confirmGroup': 'Bestätige mindestens eine Gruppe, bevor du abschließt.',
};
```

### Verification

```bash
pnpm typecheck   # catches missing or extra German keys
pnpm test -- i18n
```

---

## 11. Phase 6 — App: startup, `<html lang>`, and sidebar switcher

**Goal:** make German the first-run language, persist changes, and expose the
switcher beside the theme toggle.

### Bootstrap

```html
<!-- [MODIFY] index.html -->
<html lang="de">
  ...
  <script>
    (function () {
      try {
        var stored = localStorage.getItem('topdata.theme.v1');
        document.documentElement.dataset.theme =
          stored === 'dark' || stored === 'light' ? stored : 'light';

        var language = localStorage.getItem('topdata.language.v1');
        document.documentElement.lang = language === 'en' ? 'en' : 'de';
      } catch (error) {
        document.documentElement.dataset.theme = 'light';
        document.documentElement.lang = 'de';
      }
    })();
  </script>
```

```ts
// [MODIFY] src/main.ts — import the runtime before mounting
import './lib/i18n';
```

### Sidebar

```vue
<!-- [MODIFY] src/components/Sidebar.vue — script -->
<script setup lang="ts">
import {
  TdLanguageSwitcher,
  type TdLanguageOption,
} from '@topdata-software-gmbh/ui-kit-web-vue3';
import { language, setLanguage, t } from '../lib/i18n';

const languageOptions: TdLanguageOption[] = [
  { value: 'de', label: 'Deutsch', short: 'DE' },
  { value: 'en', label: 'English', short: 'EN' },
];
</script>
```

```vue
<!-- [MODIFY] src/components/Sidebar.vue — footer, between Look again and theme -->
<TdLanguageSwitcher
  :model-value="language"
  :languages="languageOptions"
  :aria-label="t('sidebar.changeLanguage')"
  @update:model-value="setLanguage"
/>
```

### Verification

- Fresh profile (clear `localStorage`): UI is German, `<html lang="de">`.
- Switch to English: labels change immediately, `<html lang="en">`, reload keeps English.
- Switch back to German: persists.

---

## 12. Phase 7 — App: translate shell and table components

**Goal:** replace every literal in the main workspace with `t()`.

### Component key mapping

Use the catalogue keys from Phase 4. The mapping below is the checklist.

#### `App.vue`

```ts
// [MODIFY] src/App.vue — script
import { t } from './lib/i18n';

// inside onMounted catch:
store.showToast('error', t('app.liveUpdatesUnavailable.title'), String(error));
```

#### `Sidebar.vue`

- `confirm(...)` → `t('sidebar.confirmRemoveFolder', { path })`.
- All nav labels, section headers, `title`/`aria-label`, empty hints, theme
  labels, more-menu items, and the language label use the `sidebar.*` keys.
- Product name `Git Repo Manager` stays untranslated.
- Confirm that `All projects` now matches the English README wording; the German
  string is `Alle Projekte`.

#### `Toolbar.vue`

- Count badge: `tPlural('common.project', count)`.
- `cleanRepositoryCount` copy: `t('toolbar.upToDateCount', { count })`.
- Branch options: `t('toolbar.noProjectsYet')`, `t('toolbar.chooseBranch')`.
- Action labels: `t('toolbar.saveToGit')`, `t('toolbar.getLatest')`,
  `t('toolbar.sendChanges')`, and the `toolbar.*` menu keys.
- Every `title`, `aria-label`, spinner label, and description uses a key.
- `runAction` calls `store.executeGroupAction(action)` unchanged; copy lives in the
  store after Phase 9.

#### `RepoTable.vue`

- Search placeholder/label, result counter, all four empty-state blocks, and their
  buttons use `empty.*` keys.
- Interpolate the group name into `empty.noProjectsInGroup.text`:
  `t('empty.noProjectsInGroup.text', { group: store.currentGroup?.name ?? '' })`.

#### `RepoListTable.vue`

- Headers, `sr-only` actions label, detached warning, branch hint, checkpoint empty
  text, row action labels/hints, and `aria-label` interpolation use `table.*`.
- `statusBadges()` returns translated labels after Phase 9, so the existing icon
  condition that compares `badge.label === 'Needs attention'` must be replaced with
  the stable `kind`:

```vue
<!-- [MODIFY] src/components/RepoListTable.vue -->
<AlertCircle v-if="badge.kind === 'attention'" />
<GitMerge v-else-if="badge.kind === 'operation'" />
```

#### `AllReposPage.vue`

- Move `statusOptions` and `kindOptions` into `computed`s that call `t()` so they
  re-render on language change.
- Group options: `t('filters.anyGroup')`, `t('filters.ungrouped')`.
- All filter labels, empty states, scanning copy, and buttons use `allRepos.*`,
  `filters.*`, and `empty.*`.
- `folderLabel` fallbacks use `t('allRepos.yourFolders')` /
  `t('allRepos.thatFolder')`.

```ts
// [MODIFY] src/components/AllReposPage.vue — representative
const statusOptions = computed<Array<{ value: RepoStatusFilter; label: string }>>(() => [
  { value: 'needsAttention', label: t('filters.needsAttention') },
  { value: 'changesNotSaved', label: t('filters.changesNotSaved') },
  { value: 'updatesAvailable', label: t('filters.updatesAvailable') },
  { value: 'toSend', label: t('filters.toSend') },
  { value: 'upToDate', label: t('filters.upToDate') },
  { value: 'localOnly', label: t('filters.localOnly') },
]);
```

> The filter-chip labels must not include a count in their text because
> `TdSegmentedControl` renders the count separately. Use the dedicated label keys
> (`filters.needsAttention`, `filters.changesNotSaved`, `filters.updatesAvailable`,
> `filters.toSend`, `filters.upToDate`, `filters.localOnly`) instead of the status
> badge templates. Add these six keys to both catalogues in Phases 4–5.

#### `GuidanceBanner.vue`

- No literals; the store supplies translated `insight.title` / `insight.message`.

### Verification

```bash
pnpm typecheck
pnpm test
```

Manually inspect the main workspace in German and English for truncation and
layout shifts (German is often ~20–35 % longer).

---

## 13. Phase 8 — App: translate modals, drawers, and onboarding

**Goal:** externalise every string in dialogs and the first-run flow.

### `HelpModal.vue`

- Convert `steps` and `words` arrays into `computed`s that call `t()` so they react
  to a language switch.
- Modal title/description, section heading, warning, and footer button use
  `help.*`.

```ts
// [MODIFY] src/components/HelpModal.vue — representative
const steps = computed(() => [
  { title: t('help.step1.title'), text: t('help.step1.text') },
  { title: t('help.step2.title'), text: t('help.step2.text') },
  { title: t('help.step3.title'), text: t('help.step3.text') },
  { title: t('help.step4.title'), text: t('help.step4.text') },
]);

const words = computed(() => [
  { icon: Save, name: t('help.word.save.name'), text: t('help.word.save.text') },
  { icon: Download, name: t('help.word.pull.name'), text: t('help.word.pull.text') },
  { icon: Upload, name: t('help.word.push.name'), text: t('help.word.push.text') },
  { icon: GitBranch, name: t('help.word.branch.name'), text: t('help.word.branch.text') },
  { icon: Archive, name: t('help.word.stash.name'), text: t('help.word.stash.text') },
]);
```

### `GroupEditorModal.vue`

- Validation `error.value = t('group.nameRequired')`.
- Delete confirm: `t('group.deleteConfirm', { name: group.name })`.
- Modal title/description, form labels, colour aria label, select-all/clear-all,
  empty projects text, selected count, advanced settings, checkbox
  labels/descriptions, base-branch label, and footer buttons use `group.*` /
  `common.*`.
- Selected count:
  `t('group.selectedCount', { count, total, projectWord: tPlural('common.project', total) })`.

### `AddScanPathModal.vue`

- Title, form labels/hints, advanced summary, cancel, and submit use `scan.*`.

### `SaveChangesModal.vue`

- `statusLabel` maps to `save.status.*`.
- Error: `t('save.couldNotRead', { names: failed.join(', ') })`.
- Success toast: `t('save.savedTitle')` and
  `t('save.savedBody', { count, projectWord: tPlural('common.project', count) })`.
- Loading copy, working-on line, changed badge, form label/hint/placeholder, totals,
  and footer buttons use `save.*`.
- Totals:
  `t('save.total', { files, fileWord: tPlural('common.file', files), projects, projectWord: tPlural('common.project', projects) })`.

```ts
// [MODIFY] src/components/SaveChangesModal.vue — representative
function statusLabel(status: FileChangeStatus): string {
  switch (status) {
    case 'new':
      return t('save.status.new');
    case 'deleted':
      return t('save.status.deleted');
    case 'renamed':
      return t('save.status.renamed');
    case 'conflict':
      return t('save.status.conflict');
    default:
      return t('save.status.changed');
  }
}
```

### `GitLogDrawer.vue`

- Translate only the drawer chrome (`gitLog.*`): region/title, close, empty state,
  shop count, show/hide commands, "not needed this time".
- Backend `run.title`, `row.comment`, `row.command`, `row.alias`, and `row.message`
  stay as delivered. Add a source comment:

```ts
// Git action titles/comments come from src-tauri/macros/git-actions.yaml and
// stay English in this increment; see the plan's scope boundary.
```

### Onboarding

#### `OnboardingWizard.vue`

- Steps become computed labels from `onboarding.step.*`; counter uses
  `onboarding.stepCounter`; title, close aria, and progress aria use `onboarding.*`.

#### `OnboardingStepFind.vue`

- Grouping options become `computed`; labels/hints/scanning copy/submit use
  `onboarding.find.*`.

#### `OnboardingStepChoose.vue`

- `kindLabel` maps to `filters.theme` / `filters.plugin` / `filters.other`.
- Search, selection buttons, count, empty states, back, and continue use
  `onboarding.choose.*` + `common.back` / `common.continue`.

#### `OnboardingStepGroups.vue`

- Validation messages map to `onboarding.groups.error*`.
- `repoSummary` suffix uses `t('onboarding.groups.more', { count })`.
- Title/text, accepted count, aria labels, project count badge, empty state, back,
  and finish use `onboarding.groups.*`.

```ts
// [MODIFY] src/components/onboarding/OnboardingStepGroups.vue — representative
function repoSummary(paths: string[]): string {
  const names = paths.slice(0, 3).map(repoName);
  const suffix = paths.length > 3
    ? t('onboarding.groups.more', { count: paths.length - 3 })
    : '';
  return names.join(', ') + suffix;
}

const acceptedLabel = computed(() =>
  t('onboarding.groups.accepted', {
    count: store.onboardingSelectedRepositories.length,
    projectWord: tPlural('common.project', store.onboardingSelectedRepositories.length),
  }),
);
```

#### `OnboardingStepReady.vue`

- Summary:
  `t('onboarding.ready.summary', { projects, projectWord, groups, groupWord })`.
- Card titles/texts, open-workspace section, group buttons, and "Open All projects"
  use `onboarding.ready.*`.

### Verification

```bash
pnpm typecheck
pnpm test
```

Walk through onboarding in both languages. Pay attention to the modal footer on
narrow widths and to long German button labels ("Einrichtung abschließen").

---

## 14. Phase 9 — App: translate store, status helpers, and onboarding helpers

**Goal:** dynamic copy reacts to language changes and no English leaks remain in
store-generated messages.

### `src/lib/repoStatus.ts`

Add a stable `kind` field so icons do not depend on translated labels.

```ts
// [MODIFY] src/lib/repoStatus.ts
import { t, tPlural } from '../i18n';
import type { Repository } from '../types/domain';

export type StatusBadgeVariant = 'neutral' | 'success' | 'info' | 'warning' | 'error';
export type StatusBadgeKind = 'attention' | 'operation';

export interface StatusBadge {
  label: string;
  variant: StatusBadgeVariant;
  kind?: StatusBadgeKind;
}

export function statusBadges(repo: Repository): StatusBadge[] {
  if (!repo.isAccessible) {
    return [{ label: t('status.folderNotFound'), variant: 'error' }];
  }

  const badges: StatusBadge[] = [];

  if (repo.state.hasConflicts) {
    badges.push({ label: t('status.needsAttention'), variant: 'error', kind: 'attention' });
  }

  if (repo.state.operationState !== 'idle') {
    badges.push({
      label: operationLabel(repo.state.operationState),
      variant: 'warning',
      kind: 'operation',
    });
  }

  if (!repo.state.isClean) {
    badges.push({ label: t('status.changesNotSaved'), variant: 'warning' });
  }

  if (repo.state.behindCount > 0) {
    badges.push({
      label: t('status.updatesAvailable', {
        count: repo.state.behindCount,
        updateWord: tPlural('common.update', repo.state.behindCount),
      }),
      variant: 'info',
    });
  }

  if (repo.state.aheadCount > 0) {
    badges.push({
      label: t('status.toSend', { count: repo.state.aheadCount }),
      variant: 'success',
    });
  }

  if (repo.state.remotes.length === 0) {
    badges.push({ label: t('status.localOnly'), variant: 'neutral' });
  } else if (repo.state.isClean && repo.state.aheadCount === 0 && repo.state.behindCount === 0) {
    badges.push({ label: t('status.upToDate'), variant: 'success' });
  }

  return badges;
}

export function operationLabel(operation: string): string {
  switch (operation) {
    case 'merging':
      return t('status.mergeInProgress');
    case 'rebasing':
      return t('status.rebaseInProgress');
    case 'cherry-picking':
      return t('status.cherryPickInProgress');
    default:
      return t('status.operationInProgress');
  }
}

export function branchLabel(repo: Repository): string {
  return repo.state.isDetachedHead
    ? t('status.notOnBranch')
    : repo.state.currentBranch || t('status.unknownBranch');
}
```

### `src/lib/onboarding.ts`

Keep the helper pure but allow a translated fallback group name:

```ts
// [MODIFY] src/lib/onboarding.ts
export function suggestGroups(
  repoPaths: string[],
  scanRoot: string,
  style: GroupingStyle,
  fallbackRootName = 'Projects',
): SuggestedGroup[] {
  const root = normalizePath(scanRoot);
  const rootName = baseName(root) || fallbackRootName;
  // ...rest unchanged
}
```

The store passes `t('onboarding.projectsGroup')` as the fourth argument. Existing
tests keep calling the three-argument form and still pass.

### `src/stores/repoStore.ts`

**1. Import and remove `ACTION_LABELS`.**

```ts
// [MODIFY] src/stores/repoStore.ts
import { t, tPlural, type MessageKey } from '../lib/i18n';

type GroupAction = 'fetch' | 'pull' | 'push' | 'stash' | 'stash_pop';

const ACTION_COPY: Record<GroupAction, { success: MessageKey; failure: MessageKey }> = {
  fetch: { success: 'toast.fetchDone', failure: 'toast.fetchFailure' },
  pull: { success: 'toast.pullDone', failure: 'toast.pullFailure' },
  push: { success: 'toast.pushDone', failure: 'toast.pushFailure' },
  stash: { success: 'toast.stashDone', failure: 'toast.stashFailure' },
  stash_pop: { success: 'toast.stashPopDone', failure: 'toast.stashPopFailure' },
};
```

> The catalogues in Phases 4–5 already include `toast.*Failure`,
> `toast.branchSwitchFailure`, `common.unknownError`, and the revised
> `toast.failedCount` template.

**2. Replace `summarizeResults`.**

```ts
// [MODIFY] src/stores/repoStore.ts
function summarizeResults(
  results: OperationResult[],
  successTitle: string,
  failureTitle: string,
): void {
  if (results.length === 0) return;

  const failures = results.filter((result) => !result.success);
  if (failures.length === 0) {
    showToast(
      'success',
      successTitle,
      t('toast.resultCount', {
        count: results.length,
        projectWord: tPlural('common.project', results.length),
      }),
    );
    return;
  }

  const details = failures
    .slice(0, 4)
    .map((failure) => {
      const firstLine = (failure.error ?? t('common.unknownError')).trim().split('\n')[0];
      return `${repoLabel(failure.repoPath)}: ${firstLine}`;
    })
    .join('\n');
  const suffix =
    failures.length > 4 ? `\n${t('common.andMore', { count: failures.length - 4 })}` : '';

  showToast(
    'error',
    t('toast.failedCount', {
      title: failureTitle,
      failed: failures.length,
      total: results.length,
    }),
    details + suffix,
  );
}
```

**3. Update every call site.**

```ts
// executeGroupAction / runRepoAction
summarizeResults(
  results,
  t(ACTION_COPY[action].success),
  t(ACTION_COPY[action].failure),
);

// switchGroupBranch
summarizeResults(
  results,
  t('toast.branchSwitched'),
  t('toast.branchSwitchFailure'),
);
```

**4. Replace simple toast strings.**

```ts
showToast('error', t('toast.loadConfigFailed'), String(error));
showToast('error', t('toast.scanFailed'), String(error));
showToast('error', t('toast.branchesFailed'), String(error));
showToast('error', t('toast.saveGroupFailed'), String(error));
showToast('success', t('toast.groupSaved'), group.name);
showToast('success', t('toast.groupDeleted'));
showToast('error', t('toast.deleteGroupFailed'), String(error));
showToast('success', t('toast.scanPathAdded'), scanPath.path);
showToast('error', t('toast.addScanPathFailed'), String(error));
showToast('success', t('toast.scanPathRemoved'));
showToast('error', t('toast.removeScanPathFailed'), String(error));
showToast('error', t('toast.saveSetupFailed'), String(error));
showToast('success', t('toast.setupComplete'), t('toast.groupsReady', {
  count,
  groupWord: tPlural('common.group', count),
}));
showToast('error', t('toast.finishSetupFailed'), String(error));
```

**5. Translate the `insight` computed.**

Replace each literal with the matching `insight.*` key. Example for the dirty
branch:

```ts
const dirty = count((repo) => !repo.state.isClean);
if (dirty > 0) {
  return {
    tone: 'warning',
    title: t(dirty === 1 ? 'insight.dirty.one' : 'insight.dirty.other', { count: dirty }),
    message: t('insight.dirty.text'),
    action: 'save',
    actionLabel: t(dirty === 1 ? 'insight.dirty.actionOne' : 'insight.dirty.actionOther'),
  };
}
```

Apply the same pattern to `missing`, `conflicts`, `busy`, `detached`, `behind`,
`ahead`, `withoutRemote`, and the up-to-date fallback. Remove the now-unused
`projectWord` helper.

**6. Translate validation errors.**

```ts
onboardingError.value = t('validation.chooseFolder');
onboardingError.value = t('validation.scanFolderFailed');
onboardingError.value = t('validation.chooseProject');
onboardingError.value = t('validation.keepGroup');
onboardingError.value = t('validation.nameEveryGroup');
onboardingError.value = t('validation.uniqueGroupNames');
onboardingError.value = t('validation.confirmGroup');
```

**7. Pass the translated fallback group name.**

```ts
suggestGroups(
  onboardingSelectedPaths.value,
  onboardingDraft.value.scanPath,
  onboardingDraft.value.groupingStyle,
  t('onboarding.projectsGroup'),
)
```

### Verification

```bash
pnpm typecheck
pnpm test
```

Open a group with mixed states and switch language while the insight banner is
visible: the banner must update immediately. Push a toast, switch language, and
confirm the visible toast keeps its old language (expected).

---

## 15. Phase 10 — Tests, regression updates, and manual verification

**Goal:** keep the existing suite green and add coverage for the new runtime.

### Update existing status tests

```ts
// [MODIFY] src/lib/__tests__/repoStatus.test.ts
import { beforeEach, describe, expect, it } from 'vitest';
import { setLanguage } from '../i18n';
import { statusBadges } from '../repoStatus';

beforeEach(() => {
  setLanguage('en');
});

// keep existing assertions; add one German check:
it('translates badges after a language switch', () => {
  setLanguage('de');
  expect(statusBadges(repo())[0]).toMatchObject({
    label: 'Nur auf diesem Computer',
    variant: 'neutral',
  });
});
```

### Manual checks

1. Fresh profile: German UI, `<html lang="de">`, sidebar switcher shows `DE`.
2. Switch to English: every shell, table, modal, onboarding step, and store banner
   updates without reload; `<html lang="en">`.
3. Reload: English persists.
4. Open each modal in German and check for clipped footer buttons.
5. Trigger a toast in German, switch language, confirm the toast remains as created
   while the rest of the UI updates.
6. Trigger each insight state if practical (missing folder, dirty repo, behind,
   ahead, local-only, up-to-date) and confirm German copy.
7. Run the full onboarding flow in German.

### Full verification

```bash
cd /topdata/topdata-design-system
pnpm typecheck
pnpm test
pnpm build

cd /topdata/repoman
pnpm typecheck
pnpm test
pnpm build
```

All four commands must pass with zero failures.

---

## 16. Phase 11 — Documentation, ADR, changelog, and housekeeping

**Goal:** document the feature and record the durable boundary.

### ADR

```md
<!-- [NEW FILE] _ai/technical_decisions/ADR__261031-1__app-owned-translations-and-dictionary-free-kit.md -->
---
title: App-owned translations with a dictionary-free UI kit
status: Accepted
date: 2026-10-31
deciders: TopData Team
tags: [i18n, design-system, ui, vue, localization]
adrId: 261031-1
---

# App-owned translations with a dictionary-free UI kit

## Context

The shared Vue kit now ships `useLanguage` and `TdLanguageSwitcher`, and
repo-manager needs a German UI with English fallback. The kit could either bundle
dictionaries or stay presentational.

## Decision

1. The kit owns language state/persistence and the switcher presentation, but no
   copy.
2. Each app owns a typed translation catalogue. Repo-manager uses a small
   `t()`/`tPlural()` runtime instead of `vue-i18n` for its two locales.
3. Language is stored locally under `topdata.language.v1`; backend/config
   persistence is deferred.
4. Backend-provided technical text (Git transcript, raw error details) is out of
   scope for this localization pass.

## Consequences

Positive: the kit stays reusable and dependency-free; key parity is enforced at
compile time; no YAML/backend drift.

Negative: a third-party consumer still has to bring its own catalogue; kit-internal
accessible labels remain English until label-override props are added.

## Alternatives

| Alternative | Rejected because |
|---|---|
| Ship en/de dictionaries in the kit | Couples the kit to two locales and every consumer pays for copy it may not use. |
| Adopt `vue-i18n` now | Unnecessary runtime/plugin weight for two locales and one app. |
| Persist language in `AppConfig` | Requires Rust schema/migration work and async writes for no current requirement. |
| Translate the Git transcript via YAML language fields | Doubles the backend contract and risks drift between YAML and catalogue. |
```

```md
<!-- [MODIFY] _ai/technical_decisions/README.md — append to 2026 index -->
- [ADR__261031-1__app-owned-translations-and-dictionary-free-kit.md](ADR__261031-1__app-owned-translations-and-dictionary-free-kit.md) — App-owned translations with a dictionary-free UI kit.
```

### Repo-manager README

Add a section after "Design system":

```md
## Languages

The UI ships in German (default) and English. The chosen language is stored under
`topdata.language.v1` in `localStorage` and applied to `<html lang>`; switch it from
the sidebar footer next to the theme toggle. The translation runtime lives in
`src/lib/i18n/`: `messages.en.ts` is the source of truth and `messages.de.ts` is
typed against it, so a missing German key fails `pnpm typecheck`. Add user-facing
copy there and call `t('namespace.key')` rather than writing literals.

The Rust backend and the "What Git did" transcript from
`src-tauri/macros/git-actions.yaml` stay English in this pass.
```

### Changelogs

```md
<!-- [MODIFY] CHANGELOG.md — Unreleased / Added -->
- German UI with an English fallback and a sidebar language switcher. The choice
  is stored under `topdata.language.v1` and applied to `<html lang>`.
- App-owned typed translation catalogues (`src/lib/i18n/`) with `t()` and
  `tPlural()`.
- Shared `useLanguage` composable and `TdLanguageSwitcher` component added to
  `@topdata-software-gmbh/ui-kit-web-vue3`.
```

```md
<!-- [MODIFY] /topdata/topdata-design-system/CHANGELOG.md — Unreleased / Added -->
- `useLanguage` composable for persisted language preference and `<html lang>` sync.
- `TdLanguageSwitcher` compact language menu component.
```

### Housekeeping

- `.gitignore`: no new file types or build artifacts are introduced; confirm no
  changes are needed in either repo.
- `README.md` (kit): updated in Phase 3.
- `README.md` (repo-manager): updated above.
- `_ai/specs/index.md` (kit): updated in Phase 3.
- `packages/ui-component-registry.yaml`: `updated_at` refreshed in Phase 3.
- Do **not** edit `_ai/conventions.md`; it is generated by `ruleman export`.

---

## 17. Phase 12 — Implementation report

**Goal:** record what was built, any deviations, and how to verify it.

Write `_ai/backlog/reports/{YYMMDD_HHmm}__IMPLEMENTATION_REPORT__language-switcher-and-german-ui.md`
using the implementation time, not the plan time.

````md
---
filename: "_ai/backlog/reports/{YYMMDD_HHmm}__IMPLEMENTATION_REPORT__language-switcher-and-german-ui.md"
title: "Report: Language switcher and German UI (design system + repo-manager)"
createdAt: YYYY-MM-DD HH:mm
updatedAt: YYYY-MM-DD HH:mm
planFile: "_ai/backlog/active/261031_2243__IMPLEMENTATION_PLAN__language-switcher-and-german-ui.md"
specFile: ""
project: "repo-manager"
status: completed
filesCreated: 0
filesModified: 0
filesDeleted: 0
tags: [i18n, german, vue, design-system, accessibility]
documentType: IMPLEMENTATION_REPORT
---

# Report: Language switcher and German UI

## Summary

<!-- 2–3 sentences: kit primitives, app catalogue, German default, switcher. -->

## Files Changed

### Created
<!-- list every new kit/app/doc file with a one-line description -->

### Modified
<!-- list every modified file with a one-line summary -->

### Deleted
<!-- "None" unless something was removed -->

## Key Changes

<!-- bullets: useLanguage, TdLanguageSwitcher, typed catalogues, t/tPlural,
     startup bootstrap, component/store translations, tests, docs -->

## Deviations from Plan

<!-- record any string that could not be translated, layout fixes forced by
     German length, or test changes that differed from the plan -->

## Technical Decisions

<!-- confirm or amend D1–D10; note any catalogue keys added during implementation -->

## Testing Notes

```bash
cd /topdata/topdata-design-system && pnpm typecheck && pnpm test && pnpm build
cd /topdata/repoman && pnpm typecheck && pnpm test && pnpm build
```

<!-- plus the manual checklist from Phase 10 -->

## Usage Examples

<!-- optional: how to add a string, how to switch language, storage key -->

## Documentation Updates

<!-- ADR, READMEs, spec, changelog, registry -->

## Next Steps

<!-- optional: backend Git transcript localization, kit label-override props,
     AppConfig persistence if language should follow the user -->
````

Fill the file counts from `git status --short` and `git diff --stat`. Do not
commit unless the user explicitly asks.
