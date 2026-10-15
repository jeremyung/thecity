# Riverbank planting candidate — 2026-10-13

The mature renderer generates no trees inside the first17m from the river edge,
while development removes nearly all farther bank trees. See
[baseline audit](riverbank-planting-baseline.json) and
[matched baseline metadata](screenshots/riverbank-planting/baseline.json).

## Cross-section and decision

Offsets are outward in X from the shared river edge:

| Feature | Offset |
| --- | --- |
| Wall/coping | 0–1.6m |
| Existing unpaved bank | 1.6–5.5m |
| Railing | 5.4m |
| Promenade | 5.5–10.5m |
| Arterial centerline | 15m (master-plan BANK_OFFSET47 minus river half-width32) |

The first diagnostic line at3.5m failed the existing arterial clearance of11.5m.
The retained diagnostic failure remains in the baseline report. No clearance
was relaxed. Actual foliage geometry gives a maximum horizontal radius2.0596m
(near) and2.3772m (far), smaller than the first probe's conservative5.1m margin.

The new candidate adds84 positions at2.7m bank offset,18m spacing, scale1.8
(existing world transform1.0). Crown bounds leave the water and railing clear,
including the tested sloping bank. Trunks/branches clear the coping. Positions
use the existing foliage/trunk instances, materials and LOD. Existing grove RNG
sequence is unchanged. Existing paving/building/road/station/extension checks
still apply on world updates. No engine, routes, saves or authoritative land
ownership changed.

The [candidate audit](riverbank-planting-candidate.json) retains76 of84 new trees
in each of seeds87231/0/5; existing outer-bank and inland retention is unchanged.
These are world-space visibility predicates, not final screen visibility or
whole-scene collision acceptance. The curved expressway deck and every possible
construction state have not been independently audited by this script.

## Checks

- TypeScript passes.
- Ten tests pass across riverbank geometry, planting envelopes and clearance cache.
- Planting tests measure both actual foliage LODs, bound arbitrary tree rotation
  with a horizontal disk, sample that disk against sloping river edges, and measure
  actual trunk/branch geometry against coping.
- Browser source reload briefly showed slow graphics startup and recovered
  naturally. Ordinary mature import is being allowed to finish through the normal
  scheduler; no queue draining is used.

Visual comparison, construction/motion, developing-state evidence and a measured
rendering-cost comparison are pending. Treat this as a candidate until they pass.
No additional planting variants should be layered onto it before the comparison.

## First browser comparison

The ordinary import completed with all four queues zero. The restored checkpoint
used Hold low waterfront view,50mm,High WebGPU,1280×720,noon preview,DOFoff.
[Miniature candidate](screenshots/riverbank-planting/candidate-miniature.jpg) compared
with [Miniature baseline](screenshots/riverbank-planting/baseline-miniature.jpg)
shows a modest bank rhythm visible at neighborhood scale. The promenade and
bridge openings remain readable in this view. It does not establish the concepts'
layered landscape or general crossing safety. The earlier screenshot's journey
count was440; restored export shows438, so do not claim exact actor-frame identity.
Building count1839 and held framing are unchanged.

Settled UI showed567 draws,10,242,796 triangles,38textures and322shapes with all
queues zero. Baseline after style warmup showed567draws,10,255,252triangles,
42textures and352shapes. Different warmup/residency makes these unsuitable for
isolating planting cost or declaring performance acceptance. The candidate reuses
existing instance meshes; a controlled motion/performance comparison remains open.

Anime preparation completed and the [candidate capture](screenshots/riverbank-planting/candidate-anime.jpg)
shows the same added planting with no camera movement. Both styles support keeping
this small candidate for further review; neither is artistic completion. Next
verify construction/developing visibility and an ordinary camera-motion recording,
including bridge approaches, before adding more landscape detail.

## Growth and bridge-sidewalk clearance correction

The first24-checkpoint audit (three seeds at station phases0–4 plus ticks10000,
40000 and80000) found two retained crowns per mature seed overlapping a bridge's
sidewalk envelope by up to0.8772m. Existing clearance included a fixed road margin,
but not the full crown beyond the bridge sidewalk. Preserved failing evidence:
`riverbank-growth-before-bridge-clearance.json`.

The renderer now uses bridge half-width +2.5m sidewalk + scaled2.38m far-crown
radius. The broad-phase grid includes this larger envelope; other road clearances
are unchanged. The corrected [growth audit](riverbank-growth.json) has zero checked
conflicts in all24cases, with74 retained new trees at maturity and minimum checked
bridge clearance1.1228m. The rail envelope is also checked. This is conservative
horizontal-envelope evidence, not all-object collision or continuous growth proof.
The audit restores ordinary saves without edits and uses current production
visibility code and actual foliage bounds. Eight focused tests and TypeScript pass.
The earlier76-tree screenshots are superseded for crossing clearance; updated
motion evidence is still needed. Do not use them as proof of the correction.

Expanded the audit to41ordinary checkpoints by continuing each seed from tick10000
until its first unfinished bridge, then saving observed stages every10ticks until
open. Seed0's finishing stage fell between samples; no claim to have observed it.
All41cases have zero checked bridge/rail crown-envelope conflicts. Retention is
stable through each observed bridge's construction stages (80/78/80trees), rather
than clearing only when the bridge opens.

[Anime recording](recordings/riverbank-planting-anime.webm) and
[six sampled frames](screenshots/riverbank-planting/anime-motion-samples.jpg) show
an ordinary live20-second capture with an initial100px orbit from the held low
waterfront preset. All queues were zero before recording. Settings:High WebGPU,
50mm,1280×720,DOFoff,noon lighting preview,1xsimulation,16xdevelopment; source
checkpoint85169. A return drag was issued later, but decoded frames show the
initial orbit followed by a hold; do not claim a complete return orbit was recorded.
Bank trees and bridge openings remain coherent in these samples. A57ms UI longest
frame appeared during the initial orbit; later rolling windows were41–42ms. Those
are unresolved motion observations, not attributed planting cost or full-recording
performance measurements. The sequence does not prove every-frame edge stability.

Next check the corrected candidate in Miniature motion before closing this bounded
landscape slice. Broader style, day-cycle, audio and hardware gates remain open;
do not expand the planting variant set to postpone the larger fidelity review.

## Matched Miniature motion and bounded retention

[Miniature recording](recordings/riverbank-planting-miniature.webm) and
[six samples](screenshots/riverbank-planting/miniature-motion-samples.jpg) start from
the same ordinary85169save, low-waterfront preset,50mm,HighWebGPU,1280×720,
noon preview,DOFoff,1xsimulation/16xdevelopment. Allqueues0 before recording.
Initial100pxorbit followed byhold shows the same coherent planting/bridge rhythm.
Recorded styles use separate real runs; do not claim identical actor frames.
The final UI rolling window showed42ms longest frame; this is not controlled
planting cost or a recording-wide maximum. No new visible issue in sampled frames.

Retain this bounded planting correction and stop adding variants. It provides a
modest neighborhood-scale improvement with41checkpoint clearance evidence and
both-style sampled motion. Every-frame stability, all-object collision, full
construction visuals, actual day-cycle, audio and hardware acceptance remain open.
