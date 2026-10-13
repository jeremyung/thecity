import { describe, expect, it } from 'vitest';
import { ensurePresentationWorld } from '../web/src/worker/presentation-order';

describe('world before movement presentation', () => {
  it('publishes changed structural state before allowing the movement call to proceed', () => {
    const events: string[] = [];
    const token = ensurePresentationWorld('g2:1:1:1', 'g1:1:1:1', () => {
      events.push('world'); return 'g2:1:1:1';
    });
    events.push('movement');
    expect(token).toBe('g2:1:1:1');
    expect(events).toEqual(['world', 'movement']);
  });
  it('does not force a world for ordinary movement within the same generation', () => {
    expect(ensurePresentationWorld('g2:1:1:1', 'g2:1:1:1', () => { throw Error('unexpected publish'); })).toBe('g2:1:1:1');
  });
  it('requires publication again after epoch reset, even when IDs and generation repeat', () => {
    let publications = 0;
    ensurePresentationWorld('g2:1:1:1', undefined, () => { publications++; return 'g2:1:1:1'; });
    expect(publications).toBe(1);
  });
  it('preserves generations above the JavaScript safe integer limit', () => {
    let published = '';
    ensurePresentationWorld('g9007199254740993:1:1:1', 'g9007199254740992:1:1:1', () => {
      return published = 'g9007199254740993:1:1:1';
    });
    expect(published).toBe('g9007199254740993:1:1:1');
  });
  it.each([undefined, 'g1:0:0:0'])('fails closed when publication returns %s', result => {
    expect(() => ensurePresentationWorld('g2:1:1:1', undefined, () => result)).toThrow('disagree');
  });
});
