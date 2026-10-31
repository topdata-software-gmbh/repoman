import type {
  AppConfig,
  GroupingStyle,
  OnboardingState,
  Repository,
} from '../types/domain';

export interface SuggestedGroup {
  name: string;
  repoPaths: string[];
}

export function defaultOnboardingState(): OnboardingState {
  return {
    completed: false,
    started: false,
    currentStep: 0,
    scanPath: null,
    ownerNames: [],
    groupingStyle: 'topLevel',
    selectedRepoPaths: [],
  };
}

export function shouldShowOnboarding(config: AppConfig): boolean {
  const onboarding = config.onboarding ?? defaultOnboardingState();
  if (onboarding.completed) return false;
  if (onboarding.started) return true;
  return config.scanPaths.length === 0 && config.groups.length === 0;
}

export function normalizeOwnerNames(raw: string): string[] {
  return [
    ...new Set(
      raw
        .split(/[,\n]/)
        .map((owner) => owner.trim().replace(/^@/, '').toLowerCase())
        .filter(Boolean),
    ),
  ].sort();
}

export function parseRemoteOwner(remoteUrl: string): string | null {
  const trimmed = remoteUrl.trim();
  if (!trimmed) return null;

  const scpLike = trimmed.match(/^[^@\s]+@[^:\s]+:(.+)$/);
  if (scpLike) {
    return firstPathSegment(scpLike[1]);
  }

  if (!trimmed.includes('://')) return null;

  try {
    const url = new URL(trimmed);
    if (url.protocol === 'file:') return null;
    return firstPathSegment(url.pathname);
  } catch {
    return null;
  }
}

function firstPathSegment(path: string): string | null {
  const segments = path.replace(/^\/+/, '').split('/').filter(Boolean);
  return segments.length >= 2 ? segments[0].toLowerCase() : null;
}

export function ownedRepoPaths(
  repos: Repository[],
  ownerNames: string[],
): string[] {
  const owners = new Set(ownerNames.map((owner) => owner.toLowerCase()));
  if (owners.size === 0) return [];

  return repos
    .filter((repo) =>
      repo.state.remotes.some((remote) => {
        const owner = parseRemoteOwner(remote.url);
        return owner !== null && owners.has(owner);
      }),
    )
    .map((repo) => repo.path);
}

export function suggestGroups(
  repoPaths: string[],
  scanRoot: string,
  style: GroupingStyle,
  fallbackRootName = 'Projects',
): SuggestedGroup[] {
  const root = normalizePath(scanRoot);
  const rootName = baseName(root) || fallbackRootName;

  if (style === 'single') {
    return repoPaths.length > 0
      ? [{ name: rootName, repoPaths: [...repoPaths].sort() }]
      : [];
  }

  const entries = repoPaths.map((path) => ({
    path,
    segments: relativeSegments(path, root),
  }));
  const deepKeys = new Set(
    entries.filter((entry) => entry.segments.length > 1).map((entry) => entry.segments[0]),
  );
  const useTopLevel = deepKeys.size > 1;
  const buckets = new Map<string, string[]>();

  for (const entry of entries) {
    const key =
      useTopLevel && entry.segments.length > 1 ? entry.segments[0] : rootName;
    const bucket = buckets.get(key) ?? [];
    bucket.push(entry.path);
    buckets.set(key, bucket);
  }

  return [...buckets.entries()]
    .map(([name, paths]) => ({ name, repoPaths: paths.sort() }))
    .sort((a, b) => a.name.localeCompare(b.name));
}

function normalizePath(path: string): string {
  return path.replace(/\\/g, '/').replace(/\/+$/, '');
}

function relativeSegments(path: string, root: string): string[] {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath.toLowerCase().startsWith(`${root.toLowerCase()}/`)) {
    return [];
  }
  return normalizedPath.slice(root.length + 1).split('/').filter(Boolean);
}

function baseName(path: string): string {
  const parts = normalizePath(path).split('/').filter(Boolean);
  return parts[parts.length - 1] ?? '';
}
