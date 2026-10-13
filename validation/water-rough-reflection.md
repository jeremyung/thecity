# Rough planar reflection trial — 2026-10-12

## Decision

Rejected and reverted. The preceding goal turn isolated available reflected
content; the intervening PR-feedback verification did not advance the living-city
implementation. This bounded trial tested a materially different filtering method
before deciding whether to continue the water shader workstream.

The trial generated reflection mipmaps, sampled an explicit blurred level, used
existing irregular wave normals for bounded UV distortion, and separated reflected
radiance from diffuse pigment with water-like Fresnel weighting. It added no scene
pass but did add mip generation/storage. Installed Three 0.183.2 source confirms
reflector clones share the base reflection pass. The raw diagnostic remained intact.

## Evidence

Normal import of `.tools/movement-support/browser/growth-723.json`, seed87231,
tick27925,723buildings,clear weather, paused09:55,Anime,HighWebGPU,50mm,DOFoff,
1280×720. The existing Hold waterfront view preset restored the camera. All geometry
queues reached zero through normal scheduling before capture.

- [Night candidate](screenshots/water-rough-reflection/night.png),22:00preview.
- [Dusk candidate](screenshots/water-rough-reflection/dusk.png),18:00preview.
- Prior matched night baseline: [normal](screenshots/water-night-probe/normal.png).

Night remains nearly black with isolated lamp points; dusk is a flat blue field
with conspicuous cast-shadow shapes. This does not demonstrate a perceptual gain
worth keeping. No gain-tuning second attempt was made. No whole-day, other-style,
motion, fallback, or performance acceptance is claimed. TypeScript and the one
water material lifecycle test passed for the candidate; no captured browser errors.
An initial TypeScript error chaining sample().level() was corrected with typed
clone uvNode/levelNode assignment before these checks.

## Next primary intervention

Stop water-only tuning. The fixed waterfront scene exposes repeated polygonal
teeth on both banks. Current city.ts places an IcosahedronGeometry shoulder every
4.5m for the full800m river, directly at the water edge. This explains the regular
sawtooth silhouette; it is authored geometry, not water tessellation or sampling.
The bank wall and promenade also use independent rotated boxes per centerline
segment. Replace these with a continuous joined engineered-bank kit following
the same X-offset river boundary as terrain/water, with restrained coping and
retained pedestrian railings. Preserve terrain height, water width, movement
reservations and both styles. Use the same camera at noon/dusk/night; reject gaps,
corner overlaps or a new repetitive decorative fringe. This is a geometry fidelity
pass, not permission for another planting or water-gain sweep.

Original water.ts was restored byte-for-byte from the pretrial local copy,
including the existing diagnostic and all earlier uncommitted work. Goal active;
no commit, push or external PR change.

After rollback, normal723import settled with all geometry queues0. Tab22 is
paused at the waterfront preset,Anime,noonpreview,50mmDOFoff,normal water.
[Restored noon baseline](screenshots/water-rough-reflection/restored-noon.png)
records the bank silhouette before its next intervention.
