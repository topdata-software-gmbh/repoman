---
title: "App-owned translations with a dictionary-free UI kit"
status: Accepted
date: 2026-10-31
deciders: [Marc]
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
