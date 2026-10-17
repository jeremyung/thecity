import * as THREE from 'three/webgpu';
import type { Point } from '../types';

/** Build a continuous, closed bank wall along one side of the river.
 * Offsets are measured outward from the water edge in world X, matching the
 * terrain and water boundary convention. Longitudinal sections meet exactly
 * at each supplied point; only the two global ends are capped.
 */
export function createRiverbankGeometry(
  points: readonly Point[],
  halfWidth: number,
  side: -1 | 1,
  innerOffset: number,
  outerOffset: number,
  bottom: number,
  top: number,
): THREE.BufferGeometry {
  if (points.length < 2) throw new RangeError('A riverbank needs at least two points');
  if (
    ![halfWidth, innerOffset, outerOffset, bottom, top].every(Number.isFinite) ||
    !(halfWidth >= 0) ||
    !(innerOffset >= 0) ||
    !(outerOffset > innerOffset) ||
    !(top > bottom)
  ) {
    throw new RangeError('Invalid riverbank dimensions');
  }
  for (let i = 0; i < points.length; i++) {
    const p = points[i];
    if (!Number.isFinite(p.x) || !Number.isFinite(p.z) || (i > 0 && !(p.z > points[i - 1].z))) {
      throw new RangeError('Riverbank points must be finite and increase in z');
    }
  }

  const positions: number[] = [];
  const normals: number[] = [];
  const uvs: number[] = [];
  const indices: number[] = [];
  const runLength = [0];
  for (let i = 1; i < points.length; i++) {
    runLength.push(
      runLength[i - 1] + Math.hypot(points[i].x - points[i - 1].x, points[i].z - points[i - 1].z),
    );
  }
  const faces = [
    {
      corners: (p: Point) =>
        [
          [p.x + side * (halfWidth + innerOffset), bottom, p.z],
          [p.x + side * (halfWidth + innerOffset), top, p.z],
        ] as const,
      target: [-side, 0, 0] as const,
    },
    {
      corners: (p: Point) =>
        [
          [p.x + side * (halfWidth + outerOffset), bottom, p.z],
          [p.x + side * (halfWidth + outerOffset), top, p.z],
        ] as const,
      target: [side, 0, 0] as const,
    },
    {
      corners: (p: Point) =>
        [
          [p.x + side * (halfWidth + innerOffset), top, p.z],
          [p.x + side * (halfWidth + outerOffset), top, p.z],
        ] as const,
      target: [0, 1, 0] as const,
    },
    {
      corners: (p: Point) =>
        [
          [p.x + side * (halfWidth + innerOffset), bottom, p.z],
          [p.x + side * (halfWidth + outerOffset), bottom, p.z],
        ] as const,
      target: [0, -1, 0] as const,
    },
  ];

  for (let segment = 0; segment < points.length - 1; segment++) {
    const a = points[segment],
      b = points[segment + 1];
    const u0 = runLength[segment] / 8,
      u1 = runLength[segment + 1] / 8;
    for (const face of faces) {
      const [a0, a1] = face.corners(a),
        [b0, b1] = face.corners(b);
      const verts = [a0, b0, a1, b1];
      // Determine winding from the actual sloping quad, then orient every
      // face toward its intended exterior. Separate face vertices give flat normals.
      const e1 = new THREE.Vector3(b0[0] - a0[0], b0[1] - a0[1], b0[2] - a0[2]);
      const e2 = new THREE.Vector3(a1[0] - a0[0], a1[1] - a0[1], a1[2] - a0[2]);
      const normal = e1.clone().cross(e2);
      if (normal.lengthSq() < 1e-12) continue;
      normal.normalize();
      const reverse =
        normal.x * face.target[0] + normal.y * face.target[1] + normal.z * face.target[2] < 0;
      if (reverse) normal.negate();
      const base = positions.length / 3;
      for (let i = 0; i < verts.length; i++) {
        const v = verts[i];
        positions.push(v[0], v[1], v[2]);
        normals.push(normal.x, normal.y, normal.z);
        uvs.push(i % 2 === 0 ? u0 : u1, i < 2 ? 0 : 1);
      }
      indices.push(
        ...(reverse
          ? [base, base + 2, base + 1, base + 1, base + 2, base + 3]
          : [base, base + 1, base + 2, base + 1, base + 3, base + 2]),
      );
    }
  }

  // Close only the two global ends. Every interior station remains uncapped,
  // so neighboring segment surfaces meet without internal faces.
  for (const endpoint of [0, points.length - 1]) {
    const p = points[endpoint];
    const xInner = p.x + side * (halfWidth + innerOffset);
    const xOuter = p.x + side * (halfWidth + outerOffset);
    const verts = [
      [xInner, bottom, p.z],
      [xOuter, bottom, p.z],
      [xInner, top, p.z],
      [xOuter, top, p.z],
    ];
    const base = positions.length / 3;
    const normalZ = endpoint === 0 ? -1 : 1;
    for (let i = 0; i < verts.length; i++) {
      const v = verts[i];
      positions.push(v[0], v[1], v[2]);
      normals.push(0, 0, normalZ);
      uvs.push(i % 2, i < 2 ? 0 : 1);
    }
    const reverse = side !== normalZ;
    indices.push(
      ...(reverse
        ? [base, base + 2, base + 1, base + 1, base + 2, base + 3]
        : [base, base + 1, base + 2, base + 1, base + 3, base + 2]),
    );
  }

  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  geometry.setAttribute('normal', new THREE.Float32BufferAttribute(normals, 3));
  geometry.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2));
  geometry.setIndex(indices);
  geometry.computeBoundingBox();
  geometry.computeBoundingSphere();
  return geometry;
}
