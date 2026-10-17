/** Two advection phases; z is the weight of the second sample in mix(a,b,z). */
export const WATER_FLOW_CYCLE = 6;
export function waterFlowPhase(
  time: number,
  target: { set(x: number, y: number, z: number): unknown },
): void {
  const first = (((time / WATER_FLOW_CYCLE) % 1) + 1) % 1;
  target.set(first, (first + 0.5) % 1, Math.abs(2 * first - 1));
}
