import { describe, it, expect } from 'vitest';
import { cleanFallbackGroups } from './fallbackGroups';

describe('cleanFallbackGroups', () => {
  it('keeps models and priorities aligned when an empty row is removed', () => {
    // The gap used to leave the priorities of the removed rows in place, so
    // "minimax-m3.1-flash" was paired with the priority of the empty row.
    const result = cleanFallbackGroups([
      { models: ['glm-5.1', '', 'minimax-m3.1-flash'], priorities: [1, 2, 3] },
    ]);

    expect(result).toHaveLength(1);
    expect(result[0].models).toEqual(['glm-5.1', 'minimax-m3.1-flash']);
    expect(result[0].priorities).toEqual([1, 3]);
  });

  it('trims model names', () => {
    const result = cleanFallbackGroups([
      { models: [' glm-5.1 ', 'minimax-m3.1-flash '], priorities: [1, 2] },
    ]);

    expect(result[0].models).toEqual(['glm-5.1', 'minimax-m3.1-flash']);
  });

  it('drops groups with fewer than two models', () => {
    const result = cleanFallbackGroups([
      { models: ['glm-5.1'], priorities: [1] },
      { models: ['glm-5.1', 'minimax-m3.1-flash'], priorities: [1, 2] },
    ]);

    expect(result).toHaveLength(1);
    expect(result[0].models).toEqual(['glm-5.1', 'minimax-m3.1-flash']);
  });

  it('falls back to the model index when a priority is missing', () => {
    const result = cleanFallbackGroups([
      { models: ['glm-5.1', 'minimax-m3.1-flash'], priorities: [1] },
    ]);

    expect(result[0].models).toEqual(['glm-5.1', 'minimax-m3.1-flash']);
    expect(result[0].priorities).toEqual([1, 2]);
  });
});
