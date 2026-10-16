import { describe, it, expect } from 'vitest';
import {
  retainWatchSubjects,
  selectWatchSubject,
  type WatchSubject,
} from '../web/src/camera/watch-selection';
const subject = (id: string, activity = 0, importance = 5): WatchSubject => ({
  id,
  activity,
  importance,
  suitability: 'all',
});
describe('automatic watch tour', () => {
  it('visits quiet river, station and skyline subjects despite arbitrarily busy districts', () => {
    const subjects = [
      subject('district:a', 1e8),
      subject('district:b', 1e9),
      subject('waterfront'),
      subject('station:1'),
      subject('skyline:1'),
    ];
    const visited = new Set<string>();
    let current: string | undefined;
    const sequence: string[] = [];
    for (let i = 0; i < subjects.length * 2; i++) {
      const next = selectWatchSubject(subjects, current, visited, false, false)!;
      if (next.restart) visited.clear();
      visited.add(next.subject.id);
      current = next.subject.id;
      sequence.push(current);
    }
    expect(new Set(sequence.slice(0, 5)).size).toBe(5);
    expect(new Set(sequence.slice(5)).size).toBe(5);
    expect(sequence.every((id, i) => i === 0 || id !== sequence[i - 1])).toBe(true);
    expect(sequence.slice(0, 4).filter((id) => id.startsWith('district:'))).toHaveLength(1);
  });
  it('retains different subject families within the anchor budget', () => {
    const subjects = [
      ...Array.from({ length: 40 }, (_, i) => subject(`district:${i}`, 1e8)),
      subject('founder:1', 0, 10),
      subject('waterfront'),
      subject('station:1'),
      subject('skyline:1'),
    ];
    const kept = retainWatchSubjects(subjects, 32);
    expect(kept).toHaveLength(32);
    for (const id of ['founder:1', 'waterfront', 'station:1', 'skyline:1'])
      expect(kept.some((s) => s.id === id)).toBe(true);
  });
  it('handles no subjects, one subject, and a newly available subject', () => {
    expect(selectWatchSubject([], undefined, new Set(), false, false)).toBeNull();
    const a = subject('district:a');
    expect(selectWatchSubject([a], a.id, new Set([a.id]), false, false)?.subject).toBe(a);
    const b = subject('waterfront');
    expect(selectWatchSubject([a, b], a.id, new Set([a.id]), false, false)?.subject).toBe(b);
  });
});
