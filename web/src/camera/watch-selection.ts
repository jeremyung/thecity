/** Bounded, deterministic subject variety for the automatic city tour. */
export interface WatchSubject {
  id: string;
  importance: number;
  activity: number;
  suitability: 'all' | 'night' | 'rain';
}
export const watchSubjectFamily = (id: string): string => id.split(':')[0];

/** Activity expresses interest without allowing population to dominate composition. */
function interest(subject: WatchSubject): number {
  return subject.importance + Math.min(2.5, Math.log1p(Math.max(0, subject.activity)) * 0.5);
}

/** Keep a representative of each available subject family before filling the budget. */
export function retainWatchSubjects<T extends WatchSubject>(
  subjects: readonly T[],
  limit: number,
): T[] {
  if (limit <= 0) return [];
  const ranked = [...subjects].sort(
    (a, b) => interest(b) - interest(a) || a.id.localeCompare(b.id),
  );
  const kept: T[] = [],
    families = new Set<string>();
  for (const subject of ranked) {
    const family = watchSubjectFamily(subject.id);
    if (!families.has(family) && kept.length < limit) {
      kept.push(subject);
      families.add(family);
    }
  }
  for (const subject of ranked)
    if (kept.length < limit && !kept.includes(subject)) kept.push(subject);
  return kept;
}

/** Visit every retained subject once per tour, presenting new families first. */
export function selectWatchSubject<T extends WatchSubject>(
  subjects: readonly T[],
  currentId: string | undefined,
  visited: ReadonlySet<string>,
  night: boolean,
  rainy: boolean,
): { subject: T; restart: boolean } | null {
  if (!subjects.length) return null;
  const alternatives = subjects.filter(
    (subject) => subjects.length === 1 || subject.id !== currentId,
  );
  let candidates = alternatives.filter((subject) => !visited.has(subject.id));
  const restart = candidates.length === 0;
  if (restart) candidates = alternatives;
  const seenFamilies = new Set(restart ? [] : [...visited].map(watchSubjectFamily));
  const freshFamilies = candidates.filter(
    (subject) => !seenFamilies.has(watchSubjectFamily(subject.id)),
  );
  if (freshFamilies.length) candidates = freshFamilies;
  const score = (subject: T) =>
    interest(subject) +
    (night && subject.suitability === 'night' ? 1.5 : 0) +
    (rainy && subject.suitability === 'rain' ? 1.2 : 0);
  return {
    subject: [...candidates].sort((a, b) => score(b) - score(a) || a.id.localeCompare(b.id))[0],
    restart,
  };
}
