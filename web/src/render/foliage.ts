import * as THREE from 'three/webgpu';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';

/** Build a layered broadleaf crown from UV-mapped, intersecting leaf-cluster cards. */
export function createFoliageGeometry(detail: 'near' | 'far', seed = 0x41b2a7): THREE.BufferGeometry {
  const random = seededRandom(seed ^ (detail === 'near' ? 0x2f31 : 0x6ac9));
  const count = detail === 'near' ? 64 : 28;
  const parts: THREE.BufferGeometry[] = [];
  const lower = new THREE.Color('#a5adb3');
  const middle = new THREE.Color('#c9d1c3');
  const crown = new THREE.Color('#f1eed5');
  const alternate = [new THREE.Color('#b9c7be'), new THREE.Color('#ddd6ba'), new THREE.Color('#adbcc2')];

  for (let i = 0; i < count; i++) {
    // Sample a rounded ellipsoid: broad through the middle, naturally tapered
    // at the crown and lower edges without forming a conical silhouette.
    const vertical = (random() * 2 - 1) * 0.91;
    const ring = Math.sqrt(1 - vertical * vertical) * Math.sqrt(0.16 + random() * 0.84);
    const angle = random() * Math.PI * 2;
    const radial = 1.55 * ring;
    const center = new THREE.Vector3(
      Math.cos(angle) * radial * 1.04 + (random() - 0.5) * 0.2,
      3.5 + vertical * 1.15,
      Math.sin(angle) * radial * 0.98 + (random() - 0.5) * 0.2,
    );
    const variation = alternate[Math.floor(random() * alternate.length)];
    const side = detail === 'near' ? 0.8 + random() * 0.45 : 1.4 + random() * 0.4;
    const height = side * (0.78 + random() * 0.18);
    const crownOutward = new THREE.Vector3(center.x / 1.6, 0, center.z / 1.6).normalize();
    const baseAngle = Math.atan2(crownOutward.z, crownOutward.x) + (random() - 0.5) * 0.55;
    for (let plane = 0; plane < 3; plane++) {
      parts.push(makeLeafCard(center, side, height, baseAngle + plane * Math.PI / 3, crownOutward, lower, middle, crown, variation));
    }
  }

  // Interior leaf masses provide continuous coverage at distance; the
  // existing cutout cards retain a broken silhouette around this small core.
  for (const [x,y,z,sx,sy,sz] of [
    [0,3.45,0,.95,.85,.85],[-.65,3.3,.15,.8,.72,.75],
    [.6,3.35,.3,.82,.76,.75],[.1,3.45,-.65,.8,.72,.75],
    [-.15,4.0,0,.7,.65,.65],
  ]) {
    const core=new THREE.SphereGeometry(1,8,4);
    core.scale(sx,sy,sz);core.translate(x,y,z);
    const positions=core.getAttribute('position'),colors:number[]=[];
    for(let i=0;i<positions.count;i++) {
      const shade=foliageColor(positions.getY(i),lower,middle,crown,alternate[0]).multiplyScalar(.72);
      colors.push(shade.r,shade.g,shade.b);
    }
    // Core coverage bypasses the leaf mask; keep its unused UVs in range.
    core.setAttribute('uv',new THREE.Float32BufferAttribute(new Float32Array(positions.count*2).fill(.5),2));
    core.setAttribute('color',new THREE.Float32BufferAttribute(colors,3));
    core.setAttribute('canopyCore',new THREE.Float32BufferAttribute(new Float32Array(positions.count).fill(1),1));
    parts.push(core);
  }

  const merged = mergeGeometries(parts, false);
  for (const geometry of parts) geometry.dispose();
  if (!merged) throw new Error(`Could not merge ${detail} foliage clusters`);
  merged.computeBoundingSphere();
  return merged;
}

/** Tapered, slightly bending trunk with five limbs reaching into the crown. */
export function createTreeTrunkGeometry(): THREE.BufferGeometry {
  const parts: THREE.BufferGeometry[] = [];
  const trunk = [
    [0, 0, 0, 0.24, 0.205],
    [0, 1.15, 0, 0.205, 0.16],
    [-0.025, 2.3, 0.035, 0.16, 0.115],
    [0.045, 3.42, 0.01, 0.115, 0.066],
    [0.02, 4.6, -0.025, 0.066, 0.018],
  ] as const;
  for (let i = 0; i < trunk.length - 1; i++) {
    const from = new THREE.Vector3(trunk[i][0], trunk[i][1], trunk[i][2]);
    const to = new THREE.Vector3(trunk[i + 1][0], trunk[i + 1][1], trunk[i + 1][2]);
    parts.push(taperedBranch(from, to, trunk[i][3], trunk[i][4], 7));
  }

  const limbs = [
    { from: [0.0, 2.02, 0.02], to: [0.95, 3.08, 0.15], radius: 0.092 },
    { from: [0.0, 2.38, 0.02], to: [-0.88, 3.32, 0.42], radius: 0.082 },
    { from: [0.0, 2.58, 0.02], to: [0.12, 3.35, -1.02], radius: 0.076 },
    { from: [0.0, 2.8, 0.02], to: [0.88, 3.72, -0.48], radius: 0.07 },
    { from: [0.03, 3.16, 0], to: [-0.72, 4.05, -0.6], radius: 0.058 },
  ];
  // Keep a compact authored branch set; varied directions make visible gaps in the canopy.
  for (const limb of limbs) {
    parts.push(taperedBranch(
      new THREE.Vector3(limb.from[0], limb.from[1], limb.from[2]),
      new THREE.Vector3(limb.to[0], limb.to[1], limb.to[2]),
      limb.radius,
      limb.radius * 0.34,
      5,
    ));
  }

  const merged = mergeGeometries(parts, false);
  for (const geometry of parts) geometry.dispose();
  if (!merged) throw new Error('Could not merge tree trunk and branch geometry');
  merged.computeVertexNormals();
  merged.computeBoundingSphere();
  return merged;
}

function makeLeafCard(
  center: THREE.Vector3,
  width: number,
  height: number,
  angle: number,
  outward: THREE.Vector3,
  lower: THREE.Color,
  middle: THREE.Color,
  crown: THREE.Color,
  variation: THREE.Color,
): THREE.BufferGeometry {
  const side = new THREE.Vector3(-Math.sin(angle), 0, Math.cos(angle));
  const up = new THREE.Vector3(outward.x * 0.11, 1, outward.z * 0.11).normalize();
  const halfWidth = width * 0.5;
  const halfHeight = height * 0.5;
  const top = center.clone().addScaledVector(up, halfHeight).addScaledVector(outward, 0.025);
  const bottom = center.clone().addScaledVector(up, -halfHeight);
  const leftBottom = bottom.clone().addScaledVector(side, -halfWidth);
  const rightBottom = bottom.clone().addScaledVector(side, halfWidth);
  const rightTop = top.clone().addScaledVector(side, halfWidth);
  const leftTop = top.clone().addScaledVector(side, -halfWidth);
  const positions = [
    leftBottom.x, leftBottom.y, leftBottom.z,
    rightBottom.x, rightBottom.y, rightBottom.z,
    rightTop.x, rightTop.y, rightTop.z,
    leftTop.x, leftTop.y, leftTop.z,
  ];
  const colors: number[] = [];
  for (const point of [leftBottom, rightBottom, rightTop, leftTop]) {
    const color = foliageColor(point.y, lower, middle, crown, variation);
    colors.push(color.r, color.g, color.b);
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  geometry.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));
  geometry.setAttribute('uv', new THREE.Float32BufferAttribute([0, 0, 1, 0, 1, 1, 0, 1], 2));
  geometry.setAttribute('canopyCore',new THREE.Float32BufferAttribute([0,0,0,0],1));
  geometry.setIndex([0, 2, 1, 0, 3, 2]);
  const normals:number[]=[];
  for(const point of [leftBottom,rightBottom,rightTop,leftTop]){
    const normal=new THREE.Vector3(point.x*.45,Math.max(.3,point.y-2.4),point.z*.45).normalize();
    normals.push(normal.x,normal.y,normal.z);
  }
  geometry.setAttribute('normal',new THREE.Float32BufferAttribute(normals,3));
  return geometry;
}

function foliageColor(y: number, lower: THREE.Color, middle: THREE.Color, crown: THREE.Color, variation: THREE.Color): THREE.Color {
  const height = THREE.MathUtils.clamp((y - 2.15) / 3.05, 0, 1);
  const color = height < 0.58
    ? lower.clone().lerp(middle, height / 0.58)
    : middle.clone().lerp(crown, (height - 0.58) / 0.42);
  return color.lerp(variation, 0.12);
}

function taperedBranch(from: THREE.Vector3, to: THREE.Vector3, radiusBase: number, radiusTop: number, segments: number): THREE.BufferGeometry {
  const direction = new THREE.Vector3().subVectors(to, from);
  const geometry = new THREE.CylinderGeometry(radiusTop, radiusBase, direction.length(), segments, 1, false);
  geometry.applyQuaternion(new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0, 1, 0), direction.clone().normalize()));
  geometry.translate((from.x + to.x) / 2, (from.y + to.y) / 2, (from.z + to.z) / 2);
  return geometry;
}

function seededRandom(seed: number): () => number {
  let state = seed >>> 0 || 1;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return (state >>> 0) / 0x1_0000_0000;
  };
}
