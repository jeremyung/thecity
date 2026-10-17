import * as THREE from 'three';
import { type Anchor, UP, WATCH_DIRECTION } from './shot';

export function anchorDistance(camera: THREE.PerspectiveCamera, anchor: Anchor): number {
  let minimum: number;
  let maximum: number;
  let defaultRadius: number;
  if (anchor.id.startsWith('skyline')) {
    if (anchor.framePoints?.length) {
      return fitSkylineDistance(
        camera,
        anchor.framePoints,
        anchor.cameraDirection ?? WATCH_DIRECTION,
        new THREE.Vector3(anchor.x, anchor.targetY ?? 0, anchor.z),
      );
    }
    const radius = anchor.frameRadius ?? 90;
    return THREE.MathUtils.clamp(fitSphereDistance(camera, radius, 1.12), 105, 360);
  } else if (anchor.id.startsWith('crossing:') || anchor.id === 'waterfront') {
    minimum = 270;
    maximum = 380;
    defaultRadius = 86;
  } else if (anchor.id.startsWith('founder:')) {
    minimum = 78;
    maximum = 170;
    defaultRadius = 26;
  } else if (anchor.id.startsWith('station')) {
    minimum = 205;
    maximum = 300;
    defaultRadius = 50;
  } else if (anchor.id.startsWith('district:')) {
    minimum = 160;
    maximum = 205;
    defaultRadius = 46;
  } else {
    minimum = 190;
    maximum = 290;
    defaultRadius = 42;
  }
  const radius = anchor.frameRadius ?? defaultRadius;
  return THREE.MathUtils.clamp(
    Math.max(minimum, fitDistance(camera, radius, 1.17)),
    minimum,
    maximum,
  );
}

export function fitDistance(
  camera: THREE.PerspectiveCamera,
  radius: number,
  margin: number,
  direction = WATCH_DIRECTION,
): number {
  const verticalTangent = Math.tan(THREE.MathUtils.degToRad(camera.fov * 0.5));
  const horizontalTangent = verticalTangent * validAspect(camera);
  const elevation = THREE.MathUtils.clamp(direction.y, 0.1, 0.95);
  const horizontalDistance =
    (radius * Math.sqrt(1 - elevation * elevation)) / Math.max(0.01, horizontalTangent);
  const verticalDistance = (radius * elevation) / Math.max(0.01, verticalTangent);
  return Math.max(horizontalDistance, verticalDistance) * margin;
}

/** Fit a 3D subject, including its height, instead of treating it as a ground-plane map. */
export function fitSphereDistance(
  camera: THREE.PerspectiveCamera,
  radius: number,
  margin: number,
): number {
  const verticalTangent = Math.tan(THREE.MathUtils.degToRad(camera.fov * 0.5));
  const horizontalTangent = verticalTangent * validAspect(camera);
  return (radius * margin) / Math.max(0.01, Math.min(verticalTangent, horizontalTangent));
}

/** Fit every true 3D corner against the perspective frustum, including near-side depth. */
export function fitSkylineDistance(
  camera: THREE.PerspectiveCamera,
  points: THREE.Vector3[],
  direction: THREE.Vector3,
  target: THREE.Vector3,
): number {
  const verticalTangent = Math.tan(THREE.MathUtils.degToRad(camera.fov * 0.5));
  const horizontalTangent = verticalTangent * validAspect(camera);
  const view = direction.clone().negate().normalize();
  const right = new THREE.Vector3().crossVectors(view, UP).normalize();
  const cameraUp = new THREE.Vector3().crossVectors(right, view).normalize();
  const sideLimit = 0.84;
  // Leave room for the masthead and the bottom controls/caption.
  const verticalLimit = 0.68;
  let distance = 45;
  for (const point of points) {
    const offset = point.clone().sub(target);
    const towardCamera = offset.dot(direction);
    const horizontal = Math.abs(offset.dot(right));
    const vertical = Math.abs(offset.dot(cameraUp));
    distance = Math.max(
      distance,
      towardCamera + horizontal / Math.max(0.01, horizontalTangent * sideLimit),
      towardCamera + vertical / Math.max(0.01, verticalTangent * verticalLimit),
    );
  }
  // Small air around the exact frustum solution keeps antialiasing and skyline edges clear.
  return Math.min(600, distance * 1.045 + 3);
}

export function validAspect(camera: THREE.PerspectiveCamera): number {
  return Number.isFinite(camera.aspect) && camera.aspect > 0 ? camera.aspect : 1;
}

/** Engine heading is atan2(dz, dx): zero points east (+X), π/2 north (+Z). */
export function headingForward(heading: number, out: THREE.Vector3): THREE.Vector3 {
  return out.set(Math.cos(heading), 0, Math.sin(heading));
}
