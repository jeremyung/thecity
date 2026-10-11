export interface RenderStartupLifecycle {
  readonly promise: Promise<void>;
  runFrame(render: () => void): void;
  dispose(): void;
}

/** Resolves after the first completed JS render submission; it does not imply GPU presentation. */
export function createRenderStartupLifecycle(options: {
  stopLoop: () => void;
  setPhase: (phase: string) => void;
  onError: (error: Error) => void;
}): RenderStartupLifecycle {
  let startupSettled = false;
  let disposed = false;
  let failed = false;
  let firstFrame = false;
  let resolveStartup!: () => void;
  let rejectStartup!: (error: Error) => void;
  const promise = new Promise<void>((resolve, reject) => {
    resolveStartup = resolve;
    rejectStartup = reject;
  });

  return {
    promise,
    runFrame(render) {
      if (disposed || failed) return;
      try {
        render();
        if (!firstFrame) {
          firstFrame = true;
          options.setPhase('first-frame-submitted');
          startupSettled = true;
          resolveStartup();
        }
      } catch (cause) {
        const error = cause instanceof Error ? cause : new Error(String(cause));
        failed = true;
        options.stopLoop();
        options.setPhase('frame-error');
        if (!startupSettled) {
          startupSettled = true;
          rejectStartup(error);
        } else {
          options.onError(error);
        }
      }
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      if (!startupSettled) {
        startupSettled = true;
        resolveStartup();
      }
    },
  };
}
