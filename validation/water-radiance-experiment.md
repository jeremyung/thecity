# Rejected water reflection experiment — 2026-10-11

## Deficiency and bounded experiment

The mature river appears pale and flat relative to the locked concept boards.
Inspection found that existing wave/rain normals affect lighting but do not
distort the planar reflection, and the reflected scene is mixed into diffuse
pigment, receiving lighting again. The experiment reused those normals for
screen-space reflection distortion and moved reflected radiance to emission,
with bounded angle-dependent reflectance. No extra render pass, texture, asset,
simulation state or reflection resolution was added.

Installed Three.js source confirms ReflectorNode inherits TextureNode UV
sampling and defaults to screenUV.flipX(). The experiment preserved that flip,
subtracted the flat-plane view normal to avoid camera-dependent global offsets,
and clamped UVs. Existing anti-aliased fine waves and simulated rain remained.

## Two attempts and decision

1. Distortion plus direct reflected radiance compiled and rendered in WebGPU,
   but made the sky reflection dominate the river. It remained a pale mirror.
2. Lower overhead reflectance and an intermediate-scale resolved sine wave
   made the reflected bridge edges visibly wavy, but introduced conspicuous
   horizontal bands in the dusk glare. Reject this version too.

**Both attempts were reverted. `web/src/render/water.ts` is unchanged from the
pre-experiment version.** Build success and an empty browser error log do not
establish artistic quality. No water improvement or WebGL acceptance is claimed.

## Evidence and limits

Seed 87231, mature 1839-building current-format checkpoint, Anime, DOF off,
50 mm, high-detail WebGPU, clear weather, 18:00 lighting preview. Each import
used normal production scheduling. First-attempt diagnostics reached zero
building and neighborhood queues. The worlds were held in Explore after
Overview framing, but camera coordinates and baseline tick were not exported;
these are qualitative comparisons, not registered before/after benchmarks.

- [Before](screenshots/water-radiance/before-dusk.jpg)
- [First-attempt diagnostics](screenshots/water-radiance/diagnostics.jpg)
- [Rejected second version](screenshots/water-radiance/rejected-banding.jpg)

## Next action

Do not continue adjusting reflection gains blindly. Establish a repeatable
waterfront camera and lighting fixture, including a close view and distant
view, before another shader variant. Separate direct specular glare, reflected
sky and reflected lit geometry when diagnosing the pale surface. A future
multi-scale irregular wave treatment must avoid coherent stripes and remain
stable under camera movement and reduced reflection resolution. Review both
styles at noon/dusk/night and WebGL before promotion. Broader sky/light and
architectural fidelity remain open, so this experiment must not become an
unbounded water-only tuning workstream.

## Isolated coefficient-preserving trial — 2026-10-12

The preceding turn established a repeatable low-angle fixture (48 camera tests,
byte-identical browser reset), making this a controlled comparison rather than
another gain sweep. The trial moved only reflection.rgb * existing style weight
* (1 - shore weight) from colorNode into emissiveNode. Base pigment, original
reflection coefficients, shore wash, waves, rain, resolution and lifecycle were
unchanged. Luna independently checked the algebra and installed Three source;
root inspected the diff and captured the browser.

Normal full1839 import,seed87231,tick75925,clear,paused,Anime,HighWebGPU,
50mm,DOFoff,1280×720. All queues reached zero normally. Low preset
offset(130,55,215) and overhead(110,145,180), both at river midpoint.
Captures under screenshots/waterfront-growth:
- isolated-radiance-low-night.png versus low-preset-anime-night.png.
- isolated-radiance-low-dusk.png and isolated-radiance-low-noon.png.
- isolated-radiance-overhead-night.png versus current-anime-night-matched.png.

At night the candidate restores visible building/window reflections. At noon
and dusk it exposes a pale mirror with conspicuous stepped reflected edges,
which fails the visual gate. The diagnostic confirms attenuation but does not
solve water appearance. Rejected and restored water.ts byte-for-byte from the
pretrial copy. TypeScript and water lifecycle test passed for the candidate;
captured console errors/warnings empty. No second-style, fallback, motion or
performance acceptance is claimed because the first-style static gate failed.

Stop iterative full-city radiance/gain tuning. A later water redesign needs an
isolated reflective-surface benchmark that separately exercises bright sky,
emissive objects, view angle and wave response before full-city integration.
It must integrate rough reflection sampling with surface normals and angular
reflectance, not merely relocate or amplify the same flat reflection. Do not
introduce that larger redesign speculatively as another tweak.

Return the primary stream to architectural/material fidelity using the current
mature scene: large blank facades, repeated massing and sparse street frontage
remain conspicuous in both night and noon evidence. This reprioritization leaves
water explicitly unfinished; it does not weaken its release requirement.
