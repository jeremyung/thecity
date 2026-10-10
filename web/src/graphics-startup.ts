export type ErrorStateSetter = (update: string | null | ((previous: string | null) => string | null)) => void;

const STARTUP_ERROR = 'Graphics are taking longer than expected to start. Your city is still open; try Retry or Use WebGL fallback.';

/** Reports a stuck renderer start without touching simulation state or reloading the page. */
export function createGraphicsStartupWatchdog(setError: ErrorStateSetter, delayMs = 15_000) {
  let finished = false;
  let timedOut = false;
  const timer = setTimeout(() => {
    if (finished) return;
    timedOut = true;
    setError((previous) => previous ?? STARTUP_ERROR);
  }, delayMs);

  return {
    complete() {
      if (finished) return;
      finished = true;
      clearTimeout(timer);
      if (timedOut) setError((previous) => previous === STARTUP_ERROR ? null : previous);
    },
    cleanup() {
      finished = true;
      clearTimeout(timer);
    },
  };
}
