/** Camera-space-independent clearance checks for a swept path through the city. */
export interface CameraPoint3 {
  readonly x: number;
  readonly y: number;
  readonly z: number;
}

/**
 * An axis-aligned building envelope rotated around world Y. `maxY` must include
 * visible roof equipment, not only `floorCount * floorHeight`: current ordinary
 * roof kits can extend about 2.75 m above the floor line, and 12+ floor towers
 * can extend about 4.05 m (the tower aerial is included by `skylineBounds`).
 */
export interface CameraObstacleBox {
  readonly center: { readonly x: number; readonly z: number };
  readonly rotation: number;
  readonly width: number;
  readonly depth: number;
  readonly minY: number;
  readonly maxY: number;
}

export interface CameraClearanceOptions {
  /** Flat terrain floor under the camera path, in world metres. */
  readonly terrainY?: number;
  /** Inflates each local footprint side and raises the required roof clearance. */
  readonly safetyMarginM?: number;
}

export interface CameraPathClearance {
  /** Absolute world Y that clears terrain and every horizontally swept roof. */
  readonly requiredCameraHeight: number;
  /** True when the segment intersects an expanded obstacle or terrain clearance plane. */
  readonly collides: boolean;
}

type LocalPoint = { x: number; y: number; z: number };
type Interval = { enter: number; exit: number };

function finite(name: string, value: number): void {
  if (!Number.isFinite(value)) throw new RangeError(`${name} must be finite`);
}

function validatePoint(name: string, point: CameraPoint3): void {
  finite(`${name}.x`, point.x);
  finite(`${name}.y`, point.y);
  finite(`${name}.z`, point.z);
}

function validateBox(box: CameraObstacleBox): void {
  finite("obstacle.center.x", box.center.x);
  finite("obstacle.center.z", box.center.z);
  finite("obstacle.rotation", box.rotation);
  finite("obstacle.width", box.width);
  finite("obstacle.depth", box.depth);
  finite("obstacle.minY", box.minY);
  finite("obstacle.maxY", box.maxY);
  if (box.width <= 0 || box.depth <= 0 || box.minY > box.maxY) {
    throw new RangeError("obstacle dimensions must be positive and minY must not exceed maxY");
  }
}

/** Convert world XZ to the box's local frame using Three.js rotateY convention. */
function toLocal(point: CameraPoint3, box: CameraObstacleBox): LocalPoint {
  const dx = point.x - box.center.x;
  const dz = point.z - box.center.z;
  const cos = Math.cos(box.rotation);
  const sin = Math.sin(box.rotation);
  return {
    x: cos * dx - sin * dz,
    y: point.y,
    z: sin * dx + cos * dz,
  };
}

/** Closed segment/slab interval; exact tangency counts as intersection. */
function segmentBoxInterval(start: number, end: number, halfExtent: number): Interval | null {
  const delta = end - start;
  if (Math.abs(delta) < 1e-12) {
    return start >= -halfExtent && start <= halfExtent ? { enter: 0, exit: 1 } : null;
  }
  let enter = (-halfExtent - start) / delta;
  let exit = (halfExtent - start) / delta;
  if (enter > exit) [enter, exit] = [exit, enter];
  enter = Math.max(0, enter);
  exit = Math.min(1, exit);
  return enter <= exit ? { enter, exit } : null;
}

function horizontalInterval(
  start: CameraPoint3,
  end: CameraPoint3,
  box: CameraObstacleBox,
  margin: number,
): Interval | null {
  const a = toLocal(start, box);
  const b = toLocal(end, box);
  const x = segmentBoxInterval(a.x, b.x, box.width / 2 + margin);
  if (!x) return null;
  const z = segmentBoxInterval(a.z, b.z, box.depth / 2 + margin);
  if (!z) return null;
  const enter = Math.max(x.enter, z.enter);
  const exit = Math.min(x.exit, z.exit);
  return enter <= exit ? { enter, exit } : null;
}

function verticalInterval(startY: number, endY: number, minY: number, maxY: number): Interval | null {
  return segmentBoxInterval(startY - (minY + maxY) / 2, endY - (minY + maxY) / 2, (maxY - minY) / 2);
}

/** Exact 2D swept-footprint test against an oriented, locally expanded box. */
export function segmentIntersectsOrientedBoxXZ(
  start: CameraPoint3,
  end: CameraPoint3,
  box: CameraObstacleBox,
  safetyMarginM = 0,
): boolean {
  validatePoint("start", start);
  validatePoint("end", end);
  validateBox(box);
  finite("safetyMarginM", safetyMarginM);
  if (safetyMarginM < 0) throw new RangeError("safetyMarginM must not be negative");
  return horizontalInterval(start, end, box, safetyMarginM) !== null;
}

/**
 * Find a conservative absolute camera height for the whole XZ sweep and report
 * whether the actual 3D segment enters any expanded obstacle volume. This is
 * a measurement only: callers decide how to alter/ease a camera path.
 */
export function cameraPathClearance(
  start: CameraPoint3,
  end: CameraPoint3,
  obstacles: readonly CameraObstacleBox[],
  options: CameraClearanceOptions = {},
): CameraPathClearance {
  validatePoint("start", start);
  validatePoint("end", end);
  const terrainY = options.terrainY ?? 0;
  const safetyMarginM = options.safetyMarginM ?? 2;
  finite("terrainY", terrainY);
  finite("safetyMarginM", safetyMarginM);
  if (safetyMarginM < 0) throw new RangeError("safetyMarginM must not be negative");

  let requiredCameraHeight = terrainY + safetyMarginM;
  // A closed terrain half-space uses the same tangency-as-collision rule as
  // the expanded building volumes below.
  let collides = Math.min(start.y, end.y) <= terrainY + safetyMarginM;
  for (const box of obstacles) {
    validateBox(box);
    const horizontal = horizontalInterval(start, end, box, safetyMarginM);
    if (!horizontal) continue;
    requiredCameraHeight = Math.max(requiredCameraHeight, box.maxY + safetyMarginM);

    const vertical = verticalInterval(start.y, end.y, box.minY - safetyMarginM, box.maxY + safetyMarginM);
    if (vertical && Math.max(horizontal.enter, vertical.enter) <= Math.min(horizontal.exit, vertical.exit)) {
      collides = true;
    }
  }
  return { requiredCameraHeight, collides };
}
