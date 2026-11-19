---
title: "Plain-language base copy with a Git-wording overlay"
status: Accepted
date: 2026-11-19
deciders: [Marc]
tags: [i18n, ux, terminology, accessibility, personal-settings]
adrId: 261119-1
---

# Plain-language base copy with a Git-wording overlay

## Context

RepoMan serves two audiences at once. Shop operators and designers need plain
words ("Save to Git", "Get latest", "Send changes"); Git power users find those
same words imprecise and want the real terms ("Commit", "Pull", "Push"). Today
only one register exists, so one group is always mildly annoyed.

[ADR 261031-1](ADR__261031-1__app-owned-translations-and-dictionary-free-kit.md)
established app-owned, typed catalogues with a `t()`/`tPlural()` runtime, a
German/English axis, and client-local persistence (`topdata.language.v1`). The
existing copy is already plain-language throughout. There is no settings screen;
the language switcher lives in the sidebar footer.

A personal preference is therefore needed that changes **how operations are
named**, independent of which language the app speaks.

## Decision

1. **Two independent axes.** `language` (`de`/`en`) stays as it is. Add
   `wording` (`plain` | `git`) as a separate, orthogonal preference. Wording is
   not a locale and never multiplies the locale set.
2. **Plain is the base and the source of truth.** The existing full catalogues
   remain plain-language and complete for both languages. Git wording is a
   **partial overlay** keyed by message key, one per language, resolved in `t()`
   as `overlay[language][key] ?? catalogue[language][key] ?? en[key]`.
3. **Overlay scope is short labels only:** operation verbs (both the `toolbar.*`
   button keys and the `action.*` keys used by toasts/insight rows), toast
   titles, dialog titles, the affected status badges, and glossary names.
   Explanatory sentences, every `*Hint`, all error copy, and the "What Git did"
   drawer are **never** reworded and stay plain in both modes.
4. **Both key families move together.** A concept's `toolbar.*` and `action.*`
   keys, plus its toast keys, are overridden as a set; overriding one without
   the others would make a button and its toast disagree.
5. **Plain mode bridges to the real term.** Existing tooltip hints keep their
   explanation and gain the Git term as a suffix, e.g. "Upload your saved
   checkpoints to the server (git push)". In `git` mode the label already is the
   term, so the suffix is omitted. Badges and non-operation controls get no
   tooltip change.
6. **Personal, client-local, default plain.** Persisted under
   `topdata.wording.v1`, following the language key convention. Never in
   `AppConfig`, never per repository, no server sync, no auto-detection.
7. **One control, no kit change.** A compact sibling control next to
   `TdLanguageSwitcher` in the sidebar footer selects the wording. The shared kit
   component is not modified.

## Consequences

Positive:

- Both audiences get the vocabulary they think in, on the same installation.
- The base catalogue stays single-source; the overlay is ~24 keys per language,
  so drift surface is small and guarded by a parity test.
- No backend, schema, or config migration; behaviour matches how language
  already persists, including separate preferences per browser.
- The plain default keeps the beginner-first product voice intact.

Negative:

- Contributors must know that terminology changes belong in the overlay, and
  that new operation labels need entries in both key families.
- A hard-coded literal or a new key added only to one catalogue silently falls
  back to plain; the parity test is what catches it.
- Tooltip-based bridging relies on `title`, which is weak for touch and
  screen-reader users; the hint itself remains the accessible explanation.

## Alternatives Considered

| Alternative | Rejected because |
|---|---|
| Two full catalogues per wording (`en-plain`, `en-git`, …) | Duplicates every string four times and drifts on the next copy change. |
| Encode wording in the locale (`de-plain`, `de-git`) | Breaks the existing language axis and leaks wording into every stored preference. |
| Inline key variants (`action.push.git`) | Pollutes the key space and makes `MessageKey` parity checks weaker. |
| A broader "persona" setting (Beginner/Experienced) that also hides guidance or opens the Git drawer | Conflates register (what things are called) with verbosity (how much is explained). |
| Persist wording in `AppConfig` | Server-side schema work and async writes for a per-person, per-browser preference. |
| Auto-detect from the user's Git config or command usage | Magic; surprising on shared machines and impossible to explain to Sandra or Marc when it guesses wrong. |
| Put the switch inside the Help modal | The glossary is where terms are explained, not where a preference is discoverable. |

## Related Decisions

- [ADR 261031-1 — App-owned translations with a dictionary-free UI kit](ADR__261031-1__app-owned-translations-and-dictionary-free-kit.md)
