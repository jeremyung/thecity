import {
  cameraPathClearance,
  type CameraClearanceOptions,
  type CameraObstacleBox,
  type CameraPoint3,
} from "./clearance";

export interface AutomaticCameraPathOptions extends CameraClearanceOptions {
  /** Extra separation above the computed clearance plane, in world metres. */
  readonly epsilonM?: number;
}

export interface AutomaticCameraPath {
  /** `lifted` paths rise above every obstacle touched by their horizontal sweep. */
  readonly kind: "straight" | "lifted";
  /** Waypoints after the current or recovered start; contains 1–3 finite points. */
  readonly waypoints: readonly CameraPoint3[];
  readonly correctedDestination: CameraPoint3;
  /** The original start lies in an obstacle/terrain clearance region. */
  readonly recoverStart: boolean;
  /** Explicit recovery pose; its vertical move is a caller-managed exception. */
  readonly recoveryPoint?: CameraPoint3;
}

const DEFAULT_EPSILON_M = 0.25;

function point(x: number, y: number, z: number): CameraPoint3 {
  return { x, y, z };
}

function samePoint(a: CameraPoint3, b: CameraPoint3): boolean {
  return a.x === b.x && a.y === b.y && a.z === b.z;
}

function checkedEpsilon(value: number): number {
  if (!Number.isFinite(value) || value <= 0) throw new RangeError("epsilonM must be finite and positive");
  return value;
}

/**
 * Plan a direct or lifted automatic camera route without changing camera state.
 * If the start is obstructed, `recoveryPoint` describes a one-time vertical
 * recovery that callers must handle explicitly; that segment is not asserted
 * clear because it may pass through the structure containing the start.
 * Every subsequent segment represented by `waypoints` clears terrain and the
 * expanded oriented building envelopes. This planner does not animate motion.
 */
export function planAutomaticCameraPath(
  start: CameraPoint3,
  desiredEnd: CameraPoint3,
  obstacles: readonly CameraObstacleBox[],
  options: AutomaticCameraPathOptions = {},
): AutomaticCameraPath {
  const epsilonM = checkedEpsilon(options.epsilonM ?? DEFAULT_EPSILON_M);
  const terrainY = options.terrainY ?? 0;
  const safetyMarginM = options.safetyMarginM ?? 2;
  const clearanceOptions = { terrainY, safetyMarginM };

  // Reuse the clearance validator even for a direct path and the zero-length
  // start check, so malformed points/obstacles fail consistently.
  const startCheck = cameraPathClearance(start, start, obstacles, clearanceOptions);
  const recoverStart = startCheck.collides;
  const recoveryPoint = recoverStart
    ? point(start.x, startCheck.requiredCameraHeight + epsilonM, start.z)
    : undefined;
  const routeStart = recoveryPoint ?? point(start.x, start.y, start.z);
  const requestedEnd = point(desiredEnd.x, desiredEnd.y, desiredEnd.z);

  const direct = cameraPathClearance(routeStart, requestedEnd, obstacles, clearanceOptions);
  if (!direct.collides) {
    return {
      kind: "straight",
      waypoints: [requestedEnd],
      correctedDestination: requestedEnd,
      recoverStart,
      ...(recoveryPoint ? { recoveryPoint } : {}),
    };
  }

  const swept = cameraPathClearance(routeStart, requestedEnd, obstacles, clearanceOptions);
  const endPointClearance = cameraPathClearance(requestedEnd, requestedEnd, obstacles, clearanceOptions);
  const correctedDestination = point(
    requestedEnd.x,
    endPointClearance.collides
      ? Math.max(requestedEnd.y, endPointClearance.requiredCameraHeight) + epsilonM
      : requestedEnd.y,
    requestedEnd.z,
  );
  const travelHeight = Math.max(
    routeStart.y,
    correctedDestination.y,
    swept.requiredCameraHeight,
    terrainY + safetyMarginM,
  ) + epsilonM;

  const candidates = [
    point(routeStart.x, travelHeight, routeStart.z),
    point(requestedEnd.x, travelHeight, requestedEnd.z),
    correctedDestination,
  ];
  const waypoints: CameraPoint3[] = [];
  let previous = routeStart;
  for (const candidate of candidates) {
    if (samePoint(previous, candidate)) continue;
    const segment = cameraPathClearance(previous, candidate, obstacles, clearanceOptions);
    if (segment.collides) throw new Error("Unable to construct a clear automatic camera path");
    waypoints.push(candidate);
    previous = candidate;
  }

  // If the end lies below terrain clearance or within a roof envelope, the
  // final waypoint is raised. Otherwise the same candidate sequence ends at
  // the requested pose. The maximum remains three points by construction.
  if (waypoints.length < 1 || waypoints.length > 3) {
    throw new Error("Automatic camera path must contain between one and three waypoints");
  }
  return {
    kind: "lifted",
    waypoints,
    correctedDestination,
    recoverStart,
    ...(recoveryPoint ? { recoveryPoint } : {}),
  };
}
