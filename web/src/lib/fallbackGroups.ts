import type { ModelFallbackGroup } from '../types';

/**
 * Clean up the fallback groups before saving them.
 *
 * Two things matter here:
 *
 * 1. `models` and `priorities` are positional — the backend pairs them by
 *    index. The previous implementation filtered empty model names out of
 *    `models` but left `priorities` untouched, so the priorities of the
 *    removed rows stayed in place and every model after the gap was paired
 *    with the wrong priority.
 *
 * 2. Model names are trimmed. The backend matches the requested model against
 *    the group names after trimming and lowercasing, so a stray space
 *    ("glm-5.1 ") would silently prevent the fallback chain from matching.
 *
 * Groups with fewer than two models are dropped: a group with a single model
 * has no fallback target.
 */
export function cleanFallbackGroups(groups: ModelFallbackGroup[]): ModelFallbackGroup[] {
  return groups
    .map((g) => {
      const kept = g.models
        .map((model, i) => ({
          model: model.trim(),
          priority: g.priorities[i] ?? i + 1,
        }))
        .filter((entry) => entry.model !== '');

      return {
        models: kept.map((entry) => entry.model),
        priorities: kept.map((entry) => entry.priority),
      };
    })
    .filter((g) => g.models.length >= 2);
}
