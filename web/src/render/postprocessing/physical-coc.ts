import type { Node } from 'three/webgpu';
import { float } from 'three/tsl';
import { SENSOR_HEIGHT_MM } from '../../camera/lens';

export interface PhysicalCoCNodes {
  /** True signed camera-space scene depth; visible surfaces have negative Z. */
  viewZ: Node<'float'>;
  focusDistanceM: Node<'float'>;
  focalLengthM: Node<'float'>;
  fNumber: Node<'float'>;
  opticalScale: Node<'float'>;
  outputHeightPx: Node<'float'>;
  maxGatherRadiusPx: Node<'float'>;
  enabled: Node<'float'>;
}

/** Thin-lens diameter converted to signed, bounded per-pass gather radius. */
export function physicalCoCNode(input: PhysicalCoCNodes) {
  const z=input.viewZ.negate().mul(input.opticalScale);
  const s=input.focusDistanceM.mul(input.opticalScale);
  const f=input.focalLengthM;
  const valid=z.greaterThan(f).and(s.greaterThan(f));
  // f²(z-s)/(N*z*(s-f)) is signed sensor diameter in metres.
  // Keep the unselected branch finite even for sky/invalid optical depths.
  const denominator=input.fNumber.mul(z.max(.000001)).mul(s.sub(f).max(.000001));
  const diameterPx=f.mul(f).mul(z.sub(s)).div(denominator)
    .mul(1000/SENSOR_HEIGHT_MM).mul(input.outputHeightPx);
  const normalized=diameterPx.mul(.5).div(input.maxGatherRadiusPx.max(.000001))
    .clamp(-1,1).mul(input.enabled);
  return valid.select(normalized,float(0));
}
