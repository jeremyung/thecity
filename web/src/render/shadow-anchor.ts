import * as THREE from 'three/webgpu';

/** Keep the orthographic shadow texel grid fixed while the observer pans.
 * Only the two light-plane coordinates are rounded; depth and light direction
 * remain unchanged. This does not increase shadow resolution or filter quality.
 */
export function snapShadowAnchor(
  target: THREE.Vector3,
  lightOffset: THREE.Vector3,
  spanX: number,
  spanY: number,
  mapWidth: number,
  mapHeight: number,
  output: THREE.Vector3,
): THREE.Vector3 {
  const z = lightOffset.clone().normalize();
  const x = new THREE.Vector3(0, 1, 0).cross(z);
  // Match a stable alternate up vector near a vertical light direction.
  if (x.lengthSq() < 1e-10) x.set(1, 0, 0);
  else x.normalize();
  const y = z.clone().cross(x).normalize();
  const texelX = spanX / mapWidth, texelY = spanY / mapHeight;
  const tx = target.dot(x), ty = target.dot(y);
  return output.copy(target)
    .addScaledVector(x, Math.round(tx / texelX) * texelX - tx)
    .addScaledVector(y, Math.round(ty / texelY) * texelY - ty);
}
