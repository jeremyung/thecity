# Photographic depth baseline — 2026-10-12

User reports distant surfaces remain too clear. Current model is real depth-based thin-lens CoC, not a tilted focus plane. Matched toggles confirm visible blur in the office scene, but weak separation in the waterfront view. This is not locked-concept fidelity acceptance.

Evidence: screenshots/camera-dof-baseline/miniature-50-off.png and miniature-50-on.png use held overhead waterfront; office-50-off.png and office-50-on.png use the same held office view. Seed0 tick9250,260 buildings, Miniature, High,50mm f/5.6,17:00 preview, simulation paused, Explore, WebGPU. Normal import scheduler settled before capture. Viewport847×764. The waterfront preset has little developed context at this stage and is a geometry-review view, not an accepted Watch composition. Office pairs reveal softened background balconies; foreground/backdrop separation remains inadequate for the photo references. Camera framing and dark illumination are also gaps.

Illustrative optical calculation (not measured scene depth): at1528 output pixels,50mm f/5.6,Miniature scale.01 and focus70m, objects35/140/1000m have raw signed diameters -43.73/21.86/40.67px. All hit the current8px radius cap. With focus250m, objects125/500/1000m instead have -11.60/5.80/8.70px diameters, below that cap. Raising the cap therefore cannot solve all wide-view depth separation.

A radius16 candidate was tried with all other lens values unchanged, but office-50-cap16.png is INVALID as a matched comparison: the existing developer Frame review building button used animated onFollow, and manual Explore interrupted its transition at a different pose. Radius16 was reverted to8; no optical change promoted. Root is replacing that developer action with a deterministic held review pose before another optical comparison. Ordinary user follow behavior is separate.

Next: confirm repeated building review framing, then compare at exact camera/target/time, including both styles and motion; judge sharp subject, foreground/background separation, silhouette halos and highlights. Aperture/scale and tilted-plane work remain unaccepted. Growth render stalls remain outstanding and documented in contour-growth-browser.md; this camera work responds to explicit user steering.

## Repeatable comparison and composite correction

The new held building action was browser-verified: held-office-50-on.png and
held-office-50-repeat.png (after visiting the waterfront and reframing1814)
have identical SHA256 ff21154e7462489ced7995007cfb9325510c134f22f837795a500c22af6600e9.
A fresh normal import with the same held action allowed a valid cap16 comparison
(held-office-50-cap16.png), unlike the earlier invalid experiment.

The larger cap alone visibly weakened intermediate blur. Source inspection found
the composite blended beauty with defocus using min(normalizedRadius,.5)*2.
Because normalizedRadius=physicalRadius/cap, increasing a sampling cap decreased
coverage even when the actual physical sampling radius stayed identical. Candidate
correction uses min(normalizedRadius*cap/4,1), separating output-pixel coverage
from the gather cap. It reproduces the old Miniature cap8 response mathematically;
it changes Anime cap4 response and requires an explicit Anime comparison.

Current working candidate: Miniature cap16 (Anime cap4), corrected composite.
held-office-50-cap16-coverage.png restores background separation while retaining
center facade detail. Source geometry, lens50mm f/5.6, optical scale.01, target,
time, city tick and lighting remain fixed. The source header now records the
composite deviation from upstream. Normal scheduler queues were zero before
capture. WebGPU browser console had no errors; TypeScript and build pass.
This is NOT full photographic acceptance: halos, moving occluders, nighttime
bokeh, alternate lenses/resolutions and Anime remain to check. Fixed four-pixel
coverage threshold is a provisional artistic choice, not an optical law.


## Matched night comparison — 2026-10-12

Reopened the browser after the previous review tab was no longer present. Imported
`after-opening.save.json` again **while paused** to restore seed 0 / tick 9250 /
260 buildings; an initial running import was discarded before these captures.
Used Frame review building 1814, 50 mm f/5.6, High, WebGPU, 22:00 lighting preview,
Explore. Current viewport is 1280×720, so these are new within-pair comparisons,
not pixel comparisons to the earlier 847×764 shots. No geometry, camera or time
changes between the two DOF states or the style switch. Normal scheduling only.

New files in `screenshots/camera-dof-baseline/`:
- `held-office-miniature-night-on.png` / `held-office-miniature-night-off.png`
- `held-office-anime-night-wide-on.png` / `held-office-anime-night-wide-off.png`

Visual review at native size: Miniature softens the distant apartment balconies,
shop signage and isolated point light while preserving the central office facade.
Anime's difference is much subtler. Neither shows an obvious broad silhouette halo
in this still composition; this does not rule out edge leakage, temporal artifacts
or bright-point gather artifacts elsewhere. The scene remains dark, with sparse
surrounding street activity and limited tonal separation. It is not accepted as a
match to either locked concept. Increasing the blur cap alone is not the next fix.

After Anime applied, UI reported all four geometry queues zero, 102/102 shader
samples applied, High detail, and no captured browser console errors. The UI's
short settled window showed 109 fps / frame p95 10.4 ms / longest 14.7 ms; this is
not a growth-performance result or a named-hardware benchmark. No source edits or
new automated test claims in this comparison.

Next gate: motion and foreground occlusion in this candidate, followed by an
explicit subject-focus/framing assessment. Preserve sharp subject detail; abandon
stronger blur tuning if it merely conceals weak composition or produces halos.
The known ordinary-growth render stalls remain an independent unresolved gate.


## Night orbit recordings — 2026-10-12

Saved `anime-night-orbit.webm` and `miniature-night-orbit.webm` alongside the stills,
with `motion-review.html` providing style selection and an explicit Seek button.
Both capture the canvas at 2048×1152; viewport remains 1280×720. Same paused
seed0/tick9250, 22:00 preview, 50mm, High, DOF on. Each starts from review building
1814 and uses two manual orbit drags (730,380→590,390; 590,390→700,360). Gesture
coordinates match, but their timings are not deterministic; equal video timestamps
are not exact camera-pose comparisons. No growth or moving-agent acceptance.

Capture procedure correction: closing City details unmounts ReviewRecording and
cancels/discards its recording. The retained clips were recorded with the panel
open, and AX confirmed 20/19/18 seconds remaining during the drags. The panel is
not present in the canvas-only capture. An earlier mostly stationary attempt was
replaced before saving. No change to production recording behavior was made.

Reviewed decoded frames at Anime1.5s and Miniature1.5/3s, verified currentTime via
the video DOM. Saved corresponding PNGs. These show foreground blur in Miniature,
a legible central subject and much subtler Anime blur. Bright foreground windows
expand into broad luminous shapes; no obvious detached broad halo in these
samples, but this does not establish temporal stability or all-frame edge quality.
Anime playback was exercised; complete continuous visual inspection remains
outstanding. WebM metadata initially reports infinite duration, so the review
page's duration label is not usable as clip-length proof.

A Miniature capture window reported longest frame120.3ms, frame p9516.8ms with
all geometry queues zero. Capture overhead/first-use work are not isolated, so do
not attribute the spike to DOF or claim smooth camera performance. Later settled
numbers do not erase it. Prior growth render stalls remain unresolved.

Decision: keep coverage correction provisional; do not increase blur again from
these results. Next useful investigation should isolate the motion-frame spike
and review subject composition/lighting before adding further lens effects.


## Follow subject focus — 2026-10-12

Camera review found that Follow composition intentionally looks 6/8/10 metres
beyond residents/buses/trains, but the depth pass focused on that same look-ahead
point. Director now exposes a separate reusable `focusTarget`: building center,
train center at rail/body height, or the published movement position. The camera
position, orientation and look-ahead remain unchanged. Existing axial depth and
focus damping consume this point. Missing-subject grace retains the last valid
point; manual interruption/cancellation returns focus to the orbit target. Watch
and Explore focus behavior are unchanged; tilted-plane focus remains absent.

Luna implemented the director and regression tests; root inspected lifecycle and
renderer integration, then ran 68 tests across camera, focus, lenses, presentation
and deterministic review framing. All pass; TypeScript and diff whitespace pass.
This is a correctness improvement, not evidence of a major artistic improvement.

Browser smoke/pair: seed 0, tick 9250, 260 buildings, paused at 15:15, checkpoint
weather unchanged, Miniature, 50 mm, High, WebGPU, viewport 847×764, Follow the
Neighborhood Shuttle at its terminal. Normal import queues reached zero before
capture; no queue draining. `screenshots/subject-focus/train-miniature-on.png` and
`train-miniature-off.png` keep the same paused subject/camera/time. Train remains
legible; foreground roof edges soften modestly. The pair compares DOF on/off, not
old/new focus implementations. It does not establish motion or Anime acceptance.

Perceptual priority: the observed ordinary Watch neighborhood view puts large
foreground buildings across the street subject. More blur cannot fix that
composition. The concept boards arrange clear street/transit subjects with layered
context. Next bounded camera work should assess Watch subject visibility using
existing obstacle envelopes and stable shot selection, preserving relaxed motion;
do not add blanket stronger blur. Lighting/material/density gaps remain separate.
