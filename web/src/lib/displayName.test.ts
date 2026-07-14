import { describe, it, expect } from 'vitest';
import { displayName } from './displayName';

describe('displayName', () => {
  it('returns the username when set', () => {
    expect(displayName({ username: 'alice', email: 'a@b.com' })).toBe('alice');
  });

  it('falls back to email when username is null', () => {
    expect(displayName({ username: null, email: 'a@b.com' })).toBe('a@b.com');
  });

  it('falls back to email when username is empty string', () => {
    expect(displayName({ username: '', email: 'a@b.com' })).toBe('a@b.com');
  });

  it('returns empty string when both are null', () => {
    expect(displayName({ username: null, email: null })).toBe('');
  });
});
