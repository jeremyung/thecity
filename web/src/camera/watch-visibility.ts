import { cameraPathClearance, type CameraObstacleBox, type CameraPoint3 } from './clearance';

const WORLD_LIMIT = 430;
const MAX_HEIGHT = 700;
const CAMERA_MARGIN = 1.5;
const SUBJECT_MARGIN = 0.3;
const TURNS = [0, Math.PI / 6, -Math.PI / 6, Math.PI / 3, -Math.PI / 3, Math.PI] as const;

/** Choose a bounded, clear camera position that sees along an ordinary street. */
export function chooseStreetWatchPosition(
  target: CameraPoint3,
  forward: { x: number; z: number },
  preferred: CameraPoint3,
  distance: number,
  obstacles: readonly CameraObstacleBox[],
): { position: CameraPoint3; visible: boolean } {
  for (const [name, value] of Object.entries({
    'target.x': target.x, 'target.y': target.y, 'target.z': target.z,
    'forward.x': forward.x, 'forward.z': forward.z,
    'preferred.x': preferred.x, 'preferred.y': preferred.y, 'preferred.z': preferred.z,
    distance,
  })) {
    if (!Number.isFinite(value)) throw new RangeError(`${name} must be finite`);
  }
  if (distance <= 0) throw new RangeError('distance must be positive');
  const forwardLength = Math.hypot(forward.x, forward.z);
  if (forwardLength < 1e-9) throw new RangeError('forward direction must be non-zero');

  const yaw = Math.atan2(preferred.x, preferred.z);
  const horizontalPreferred = Math.hypot(preferred.x, preferred.z);
  const baseSlope = Math.max(.25, Math.min(.65, preferred.y / Math.max(1e-3, horizontalPreferred)));
  const slopes = [baseSlope, Math.max(.45, baseSlope), .65];
  const subjects = [-12, 0, 12].map(along => ({
    x: target.x + forward.x / forwardLength * along,
    y: Math.max(target.y, 1),
    z: target.z + forward.z / forwardLength * along,
  }));

  let fallback: CameraPoint3 | undefined;
  let bestVisibleCount = -1;
  for (const slope of slopes) {
    for (const turn of TURNS) {
      const horizontal = distance / Math.sqrt(1 + slope * slope);
      const position = boundedPoint({
        x: target.x + Math.sin(yaw + turn) * horizontal,
        y: target.y + slope * horizontal,
        z: target.z + Math.cos(yaw + turn) * horizontal,
      });
      // Boundary clamping must not silently turn a shallow candidate into an overhead shot.
      if ((position.y-target.y)/Math.max(.001,Math.hypot(position.x-target.x,position.z-target.z)) > .650001) continue;
      if (cameraPathClearance(position, position, obstacles, { terrainY: 0, safetyMarginM: CAMERA_MARGIN }).collides) continue;

      const visibleCount = subjects.filter(subject => !cameraPathClearance(
        subject, position, obstacles, { terrainY: 0, safetyMarginM: SUBJECT_MARGIN },
      ).collides).length;
      if (visibleCount > bestVisibleCount) {
        fallback = position;
        bestVisibleCount = visibleCount;
      }
      if (visibleCount === subjects.length) return { position, visible: true };
    }
  }

  // If no destination is safe, leave recovery to the caller's existing travel guard.
  if (!fallback) return { position: preferredBoundedPosition(target, preferred, distance), visible: false };
  return { position: fallback, visible: false };
}

function boundedPoint(point: CameraPoint3): CameraPoint3 {
  return {
    x: Math.max(-WORLD_LIMIT, Math.min(WORLD_LIMIT, point.x)),
    y: Math.max(0, Math.min(MAX_HEIGHT, point.y)),
    z: Math.max(-WORLD_LIMIT, Math.min(WORLD_LIMIT, point.z)),
  };
}

function preferredBoundedPosition(target: CameraPoint3, preferred: CameraPoint3, distance: number): CameraPoint3 {
  const length = Math.hypot(preferred.x, preferred.y, preferred.z) || 1;
  return boundedPoint({
    x: target.x + preferred.x / length * distance,
    y: target.y + preferred.y / length * distance,
    z: target.z + preferred.z / length * distance,
  });
}
