import { afterEach, describe, expect, it, vi } from 'vitest';
import { createGraphicsStartupWatchdog } from '../web/src/graphics-startup';

describe('graphics startup feedback', () => {
  afterEach(() => vi.useRealTimers());

  it('reports a slow start, then clears only its own message when graphics come up', () => {
    vi.useFakeTimers();
    let error: string | null = null;
    const watchdog = createGraphicsStartupWatchdog((update) => {
      error = typeof update === 'function' ? update(error) : update;
    });

    vi.advanceTimersByTime(15_000);
    expect(error).toContain('Graphics are taking longer');
    expect(error).toContain('Retry');

    watchdog.complete();
    expect(error).toBeNull();
    vi.advanceTimersByTime(15_000);
    expect(error).toBeNull();
  });

  it('preserves unrelated errors when the watchdog fires or late startup completes', () => {
    vi.useFakeTimers();
    let error: string | null = 'Could not save: quota exceeded';
    const watchdog = createGraphicsStartupWatchdog((update) => {
      error = typeof update === 'function' ? update(error) : update;
    });

    vi.advanceTimersByTime(15_000);
    expect(error).toBe('Could not save: quota exceeded');
    watchdog.complete();
    expect(error).toBe('Could not save: quota exceeded');

    const slow = createGraphicsStartupWatchdog((update) => {
      error = typeof update === 'function' ? update(error) : update;
    });
    error = null;
    vi.advanceTimersByTime(15_000);
    expect(error).toContain('Graphics are taking longer');
    error = 'Could not restore the saved city';
    slow.complete();
    expect(error).toBe('Could not restore the saved city');
  });

  it('cancels pending feedback on cleanup', () => {
    vi.useFakeTimers();
    let error: string | null = null;
    const watchdog = createGraphicsStartupWatchdog((update) => {
      error = typeof update === 'function' ? update(error) : update;
    });

    watchdog.cleanup();
    vi.advanceTimersByTime(30_000);
    expect(error).toBeNull();
  });
});
