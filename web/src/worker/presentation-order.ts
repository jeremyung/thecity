/** A changed geometry generation must be published before its movement. The
 * caller clears the remembered generation whenever the worker epoch changes.
 * Tokens are opaque strings, preserving the engine's full integer precision.
 */
export function ensurePresentationWorld(
  current: string,
  published: string | undefined,
  publish: () => string | undefined,
): string {
  if (!current) throw new Error('Missing engine presentation revision');
  if (current !== published && publish() !== current) {
    throw new Error('World and movement presentation revisions disagree');
  }
  return current;
}
