import {
  fwidth,
  mix,
  mx_noise_float,
  normalWorld,
  positionWorld,
  smoothstep,
  vec3,
} from 'three/tsl';

/** Surface-anchored pigment: no screen coordinates, time, or simulation randomness. */
export function paintedSurfaceTint() {
  // Long vertical strokes describe plaster rather than isotropic screen grain.
  // Both noise bands remain shared by individual meshes and spatial batches.
  const strokePosition = positionWorld.mul(vec3(1.3, 0.35, 1.3));
  const strokeVisibility = smoothstep(0.35, 1.25, fwidth(strokePosition).length()).oneMinus();
  const strokes = mx_noise_float(strokePosition).mul(0.5).add(0.5).clamp(0, 1);
  const wash = mx_noise_float(positionWorld.mul(vec3(0.12, 0.19, 0.12)))
    .mul(0.5)
    .add(0.5)
    .clamp(0, 1);
  const pigment = mix(
    vec3(0.76, 0.85, 1.015),
    vec3(1.09, 1.01, 0.87),
    smoothstep(0.23, 0.77, wash),
  );
  // Fade unresolved strokes towards their mean rather than letting distant
  // walls sparkle as the camera moves. Horizontal paving keeps a quieter wash.
  const surfacePigment = mix(vec3(1), pigment, normalWorld.y.abs().oneMinus().mul(0.8).add(0.2));
  const wallTint = surfacePigment.mul(mix(0.97, strokes.mul(0.18).add(0.88), strokeVisibility));
  // Roofs and paving need neutral, fine surface variation, not the facade's
  // broad colour wash. Fade subpixel grain to its mean at aerial distances.
  const grainPosition = positionWorld.mul(2);
  const grainVisibility = smoothstep(0.35, 1.25, fwidth(grainPosition).length()).oneMinus();
  const horizontalTint = vec3(
    mx_noise_float(grainPosition).mul(0.025).mul(grainVisibility).add(0.97),
  );
  return mix(wallTint, horizontalTint, smoothstep(0.75, 0.95, normalWorld.y.abs()));
}
