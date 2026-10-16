import { fwidth, mix, mx_noise_float, normalWorld, positionWorld, smoothstep, vec3 } from 'three/tsl';

/** Surface-anchored pigment: no screen coordinates, time, or simulation randomness. */
export function paintedSurfaceTint() {
  // Long vertical strokes describe plaster rather than isotropic screen grain.
  // Both noise bands remain shared by individual meshes and spatial batches.
  const strokePosition = positionWorld.mul(vec3(1.3, .35, 1.3));
  const strokeVisibility = smoothstep(.35, 1.25, fwidth(strokePosition).length()).oneMinus();
  const strokes = mx_noise_float(strokePosition)
    .mul(.5).add(.5).clamp(0, 1);
  const wash = mx_noise_float(positionWorld.mul(vec3(.12, .19, .12)))
    .mul(.5).add(.5).clamp(0, 1);
  const pigment = mix(vec3(.76, .85, 1.015), vec3(1.09, 1.01, .87), smoothstep(.23, .77, wash));
  // Fade unresolved strokes towards their mean rather than letting distant
  // walls sparkle as the camera moves. Horizontal paving keeps a quieter wash.
  const surfacePigment = mix(vec3(1), pigment, normalWorld.y.abs().oneMinus().mul(.8).add(.2));
  const wallTint = surfacePigment.mul(mix(.97, strokes.mul(.18).add(.88), strokeVisibility));
  // Roofs and paving need neutral, fine surface variation, not the facade's
  // broad colour wash. Fade subpixel grain to its mean at aerial distances.
  const grainPosition = positionWorld.mul(2);
  const grainVisibility = smoothstep(.35, 1.25, fwidth(grainPosition).length()).oneMinus();
  const horizontalTint = vec3(mx_noise_float(grainPosition).mul(.025).mul(grainVisibility).add(.97));
  return mix(wallTint, horizontalTint, smoothstep(.75, .95, normalWorld.y.abs()));
}
