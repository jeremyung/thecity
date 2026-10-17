/**
 * Keep the optional miniature lens useful at city-overview distances.
 * Close views retain the existing 1:100 optical scale. Beyond 120 scene
 * metres, the damped subject distance maps to 1.2 optical metres. This is
 * an artistic scale adjustment, not a physical change to the camera lens.
 * Ordinary DOF must continue to use its own style-specific scale.
 */
export function miniatureOpticalScale(focusDepth: number): number {
  if (!Number.isFinite(focusDepth) || focusDepth <= 0) return 0.01;
  return Math.min(0.01, 1.2 / focusDepth);
}
