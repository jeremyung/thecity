import * as THREE from 'three/webgpu';

/**
 * Preetham analytic daylight, ported from three.js SkyMesh's TSL shader so
 * the painted dome can borrow physically ordered colours (blue zenith, warm
 * sunward horizon at low sun) without replacing its illustrated clouds.
 * Returns tone-mapped linear RGB in 0..1 for one view direction.
 */
export function skyRadiance(sun: THREE.Vector3, view: THREE.Vector3, turbidity = 2.6, rayleigh = 1.4, mieCoefficient = 0.005, mieG = 0.8, exposure = 0.5): THREE.Color {
  const sunDir = sun.clone().normalize(), dir = view.clone().normalize();
  const totalRayleigh = [5.804542996261093e-6, 1.3562911419845635e-5, 3.0265902468824876e-5];
  const mieConst = [1.8399918514433978e14, 2.7798023919660528e14, 4.0790479543861094e14];
  const cutoffAngle = 1.6110731556870734, steepness = 1.5, EE = 1000;
  const zenithCos = THREE.MathUtils.clamp(sunDir.y, -1, 1);
  const sunE = EE * Math.max(0, 1 - Math.exp(-(cutoffAngle - Math.acos(zenithCos)) / steepness));
  const betaR = totalRayleigh.map((v) => v * rayleigh);
  const c = 0.2 * turbidity * 10e-18;
  const betaM = mieConst.map((v) => 0.434 * c * v * mieCoefficient);

  const zenithAngle = Math.acos(Math.max(0, dir.y));
  const inverse = 1 / (Math.cos(zenithAngle) + 0.15 * Math.pow(93.885 - zenithAngle * 180 / Math.PI, -1.253));
  const sR = 8.4e3 * inverse, sM = 1.25e3 * inverse;
  const cosTheta = dir.dot(sunDir);
  const rPhase = 0.05968310365946075 * (1 + Math.pow(cosTheta * 0.5 + 0.5, 2));
  const g2 = mieG * mieG;
  const mPhase = 0.07957747154594767 * (1 - g2) / Math.pow(1 - 2 * mieG * cosTheta + g2, 1.5);
  const sunsetBlend = THREE.MathUtils.clamp(Math.pow(1 - sunDir.y, 5), 0, 1);
  const out = [0, 1, 2].map((i) => {
    const fex = Math.exp(-(betaR[i] * sR + betaM[i] * sM));
    const scatter = (betaR[i] * rPhase + betaM[i] * mPhase) / (betaR[i] + betaM[i]);
    let lin = Math.pow(sunE * scatter * (1 - fex), 1.5);
    lin *= THREE.MathUtils.lerp(1, Math.pow(sunE * scatter * fex, 0.5), sunsetBlend);
    const radiance = (lin + 0.1 * fex) * 0.04 + [0, 0.0003, 0.00075][i];
    return 1 - Math.exp(-radiance * exposure);
  });
  return new THREE.Color().setRGB(out[0], out[1], out[2], THREE.LinearSRGBColorSpace);
}
