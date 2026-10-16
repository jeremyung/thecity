import { describe, expect, it, vi } from 'vitest';
import { createRenderStartupLifecycle } from '../web/src/render/render-startup';

function lifecycle() {
  const stopLoop = vi.fn();
  const setPhase = vi.fn();
  const onError = vi.fn();
  return {
    startup: createRenderStartupLifecycle({ stopLoop, setPhase, onError }),
    stopLoop,
    setPhase,
    onError,
  };
}

describe('renderer startup frame lifecycle', () => {
  it('resolves startup after one completed JS render submission and keeps rendering', async () => {
    const { startup, stopLoop, setPhase, onError } = lifecycle();
    const render = vi.fn();
    let ready = false;
    void startup.promise.then(() => {
      ready = true;
    });
    await Promise.resolve();
    expect(ready).toBe(false);
    startup.runFrame(render);
    expect(setPhase).toHaveBeenCalledWith('first-frame-submitted');
    await expect(startup.promise).resolves.toBeUndefined();
    startup.runFrame(render);

    expect(render).toHaveBeenCalledTimes(2);
    expect(stopLoop).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
  });

  it('rejects startup and stops the loop when the first frame throws', async () => {
    const { startup, stopLoop, setPhase, onError } = lifecycle();
    const failure = new Error('pipeline submission failed');

    startup.runFrame(() => {
      throw failure;
    });

    await expect(startup.promise).rejects.toBe(failure);
    expect(stopLoop).toHaveBeenCalledOnce();
    expect(setPhase).toHaveBeenCalledWith('frame-error');
    expect(onError).not.toHaveBeenCalled();
  });

  it('reports a later frame failure through onError and stops the loop', async () => {
    const { startup, stopLoop, setPhase, onError } = lifecycle();
    startup.runFrame(() => {});
    await expect(startup.promise).resolves.toBeUndefined();
    const failure = new Error('later render failed');

    startup.runFrame(() => {
      throw failure;
    });

    expect(stopLoop).toHaveBeenCalledOnce();
    expect(setPhase).toHaveBeenCalledWith('frame-error');
    expect(onError).toHaveBeenCalledWith(failure);
  });

  it('settles pending startup on disposal and ignores callbacks after disposal', async () => {
    const { startup, stopLoop, onError } = lifecycle();
    const render = vi.fn();

    startup.dispose();
    await expect(startup.promise).resolves.toBeUndefined();
    startup.runFrame(render);

    expect(render).not.toHaveBeenCalled();
    expect(stopLoop).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
  });
});
