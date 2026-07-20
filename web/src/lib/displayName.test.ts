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

  it('prefers nickname over username and email', () => {
    expect(displayName({ nickname: 'Alice', username: 'alice123', email: 'a@x.com' })).toBe('Alice');
  });

  it('falls back to username when nickname is null', () => {
    expect(displayName({ nickname: null, username: 'alice123', email: 'a@x.com' })).toBe('alice123');
  });

  it('falls back to email when nickname and username are both null', () => {
    expect(displayName({ nickname: null, username: null, email: 'a@x.com' })).toBe('a@x.com');
  });

  it('returns empty string when all three are missing', () => {
    expect(displayName({ nickname: null, username: null, email: null })).toBe('');
  });

  it('prefers nickname even when username is empty string', () => {
    expect(displayName({ nickname: 'Bob', username: '', email: 'b@x.com' })).toBe('Bob');
  });
});
