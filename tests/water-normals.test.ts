import { describe, expect, it } from 'vitest';
import { createWaterNormalData } from '../web/src/render/water-normals';

describe('procedural ripple normal map', () => {
  it('is deterministic, mostly upward, and wraps without a seam', () => {
    const size = 64, data = createWaterNormalData(size);
    expect(Array.from(createWaterNormalData(size))).toEqual(Array.from(data));
    let up = 0, interior = 0, seam = 0;
    for (let y = 0; y < size; y += 1) {
      up += data[(y * size) * 4 + 2];
      for (let x = 0; x < size; x += 1) {
        const here = data[(y * size + x) * 4], right = data[(y * size + (x + 1) % size) * 4];
        if (x === size - 1) seam = Math.max(seam, Math.abs(here - right));
        else interior = Math.max(interior, Math.abs(here - right));
      }
    }
    expect(up / size).toBeGreaterThan(200);
    // Integer wave vectors make the tile periodic: the wrap step is no
    // larger than steps inside the tile.
    expect(seam).toBeLessThanOrEqual(interior);
  });
});
