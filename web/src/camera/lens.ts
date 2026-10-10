/** Physical still-camera lens presets shared by both city styles. */
export type LensPresetId = "environment" | "neighborhood" | "detail";

export interface LensPreset {
  readonly id: LensPresetId;
  readonly label: string;
  /** Focal length in millimeters on a 24 mm-high sensor. */
  readonly focalLengthMm: number;
  /** Aperture ratio used by the depth-of-field model. */
  readonly fNumber: number;
}

export const SENSOR_HEIGHT_MM = 24;

export const LENS_PRESETS: Readonly<Record<LensPresetId, LensPreset>> = {
  environment: { id: "environment", label: "Environment · 35 mm", focalLengthMm: 35, fNumber: 8 },
  neighborhood: { id: "neighborhood", label: "Neighborhood · 50 mm", focalLengthMm: 50, fNumber: 5.6 },
  detail: { id: "detail", label: "Detail · 85 mm", focalLengthMm: 85, fNumber: 4 },
};

function positiveFinite(name: string, value: number): void {
  if (!Number.isFinite(value) || value <= 0) {
    throw new RangeError(`${name} must be a finite positive number`);
  }
}

/** Vertical field of view for a rectilinear lens, in degrees. */
export function verticalFovDegrees(focalLengthMm: number, sensorHeightMm = SENSOR_HEIGHT_MM): number {
  positiveFinite("focalLengthMm", focalLengthMm);
  positiveFinite("sensorHeightMm", sensorHeightMm);
  return 2 * Math.atan(sensorHeightMm / (2 * focalLengthMm)) * (180 / Math.PI);
}

export interface CircleOfConfusionInput {
  /** Subject and focus distances from the lens principal plane, in scene meters. */
  readonly objectDistanceM: number;
  readonly focusDistanceM: number;
  /** Optical values in millimeters and f-number. */
  readonly focalLengthMm: number;
  readonly fNumber: number;
  readonly sensorHeightMm?: number;
  readonly viewportHeightPx: number;
  /** Maximum allowed blur diameter. Defaults to min(64 px, one quarter of viewport height). */
  readonly maxBlurPx?: number;
  /** Positive scene-distance scale for miniature optics; does not change focal length or output units. */
  readonly opticalModelScale?: number;
}

/**
 * Signed thin-lens circle-of-confusion diameter in vertical output pixels.
 * Scene distances are meters; focal length and sensor height are millimeters.
 * Negative means nearer than focus, positive means farther than focus. The
 * optional positive optical model scale is applied to scene distances only.
 */
export function signedCircleOfConfusionPixels(input: CircleOfConfusionInput): number {
  const {
    objectDistanceM,
    focusDistanceM,
    focalLengthMm,
    fNumber,
    viewportHeightPx,
    sensorHeightMm = SENSOR_HEIGHT_MM,
    maxBlurPx,
    opticalModelScale = 1,
  } = input;
  positiveFinite("objectDistanceM", objectDistanceM);
  positiveFinite("focusDistanceM", focusDistanceM);
  positiveFinite("focalLengthMm", focalLengthMm);
  positiveFinite("fNumber", fNumber);
  positiveFinite("sensorHeightMm", sensorHeightMm);
  positiveFinite("viewportHeightPx", viewportHeightPx);
  if (maxBlurPx !== undefined) positiveFinite("maxBlurPx", maxBlurPx);
  positiveFinite("opticalModelScale", opticalModelScale);

  const modeledObjectDistanceM = objectDistanceM * opticalModelScale;
  const modeledFocusDistanceM = focusDistanceM * opticalModelScale;
  const focalLengthM = focalLengthMm / 1000;
  if (!Number.isFinite(modeledObjectDistanceM) || !Number.isFinite(modeledFocusDistanceM)
      || modeledObjectDistanceM <= focalLengthM || modeledFocusDistanceM <= focalLengthM) {
    throw new RangeError("objectDistanceM and focusDistanceM must exceed focal length");
  }

  const objectImageDistanceM = (focalLengthM * modeledObjectDistanceM) / (modeledObjectDistanceM - focalLengthM);
  const focusImageDistanceM = (focalLengthM * modeledFocusDistanceM) / (modeledFocusDistanceM - focalLengthM);
  const apertureDiameterM = focalLengthM / fNumber;
  const blurDiameterOnSensorM = apertureDiameterM
    * Math.abs(objectImageDistanceM - focusImageDistanceM)
    / objectImageDistanceM;
  const blurDiameterOnSensorMm = blurDiameterOnSensorM * 1000;
  const uncappedPixels = (blurDiameterOnSensorMm / sensorHeightMm) * viewportHeightPx;
  if (![objectImageDistanceM, focusImageDistanceM, uncappedPixels].every(Number.isFinite)) {
    throw new RangeError("optical values produce a non-finite circle of confusion");
  }
  const resolutionLimitPx = maxBlurPx ?? Math.min(64, viewportHeightPx * 0.25);
  const magnitude = Math.min(uncappedPixels, resolutionLimitPx, viewportHeightPx);
  return Math.sign(modeledObjectDistanceM - modeledFocusDistanceM) * magnitude;
}

/** Backwards-compatible unsigned circle-of-confusion diameter in output pixels. */
export function circleOfConfusionPixels(input: CircleOfConfusionInput): number {
  return Math.abs(signedCircleOfConfusionPixels(input));
}

/**
 * Convert a signed CoC diameter in output pixels to a bounded normalized
 * gather radius. The half factor converts diameter to radius before the
 * pass-specific pixel radius budget is applied. Near/far classification must
 * continue to use the original signed diameter.
 */
export function normalizedGatherRadiusFromDiameterPx(diameterPx: number, maxGatherRadiusPx: number): number {
  if (!Number.isFinite(diameterPx)) throw new RangeError("diameterPx must be finite");
  positiveFinite("maxGatherRadiusPx", maxGatherRadiusPx);
  return Math.min(1, Math.abs(diameterPx) * 0.5 / maxGatherRadiusPx);
}
