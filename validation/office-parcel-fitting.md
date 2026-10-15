# Office parcel fitting diagnosis — 2026-10-13

Current decision: a fitting candidate is implemented locally; finish ordinary
construction and matched before/after visual review before retaining it.

## Candidate evidence

Non-anchor offices below12floors now reserve their1.3m side allowance without the
old6m body minimum. The setback-office family supports bodies down to3.2m, retaining
its tiers on narrow generated lots. Generic small-office construction and renovation
reuse bounded massing worksite helpers rather than the detached-yard scaffold/crane.
Generic offices keep a centered entrance. Rear AC depth adapts to available setback,
keeping casing, grille, stand and pipe together with a2cm parcel margin. Other uses,
towers and anchors retain their existing sizing rules; no engine/save change.

- [Open-mesh comparison](office-parcel-fitting-comparison.json): all557offices fit,
  with0new/remaining checked parcel violations. Triangle counts change by
  -6,624/-4,152/-6,156 across seeds87231/0/5, from fewer bays on narrower facades.
  This is mesh cost, not measured frame time.
- [Stage specimens](office-parcel-stages-candidate.json):3,342meshes across survey,
  structure, surface, finishing, renovation and demolition have0checked overhangs.
  These override stage/progress on generated descriptors; they do not establish
  ordinary lifecycle appearance or live save/replay.
- Focused regression coverage includes real generated dimensions, both styles,
  centered-door ray hits, narrow setback-office preservation, and positive unmerged
  AC casing/grille/stand bounds. The initial AC test incorrectly treated merged
  vertices as fixed-size boxes; instrumentation of individual parts corrected that
  test error. Assertions were not relaxed to accept missing equipment.
  Final root verification:53tests across8files pass; TypeScript and diff checks
  pass. See [test output](office-parcel-fitting-tests.txt) and the reversible
  [candidate patch](office-parcel-fitting-candidate.patch).
- Candidate-only browser stills of ordinary seed0/tick83255 office7195, noon
  lighting preview, overcast0.32354864,50mm,High/WebGPU,1280×720,DOFoff:
  [Miniature](screenshots/office-parcel-fitting/seed0-7195-miniature.png) and
  [Anime](screenshots/office-parcel-fitting/seed0-7195-anime.png). Both use the held
  Frame review building pose; no numerical pose was exported. The narrow building
  remains readable and coherent in both. Zero geometry queues were verified before
  capture. These are not matched baseline pairs, rear-equipment inspection, motion
  or artistic acceptance. Loading briefly displayed the renderer startup timeout
  warning and then recovered without fallback; no startup-performance pass claimed.

## Ordinary construction follow-up

The extended capture helper found ordinary seed0 office6703 on a6.3795195m-wide,
18.347536m-deep parcel at tick58200, with3floors and structure progress0.6768607.
Normal development opens it100ticks later. Original live and restored engines
match at every one of those100ticks and at opening. See
[checkpoint metadata](office-fitting-construction.json). The helper now accepts
2–11floor targets and an optional maximum parcel width; a bounded80000tick search
is used for width-filtered searches. Unfiltered defaults remain unchanged.

The Anime20second live recording has ten decoded samples at0.1/0.5/1/1.5/2/3/5/8/
12/19seconds. They show scaffolding, partial glazing, then completed ribbon facade
and rooftop, without an obvious fitting discontinuity in those samples. Both the
checkpoint and recording use normal scheduling and development rate16, simulation
speed1×. Lighting is a noon preview, not a real day-cycle check. See
[Anime contact sheet](screenshots/office-parcel-fitting/construction-anime-sheet.png)
and [recording](recordings/office-fitting-construction-anime.webm).
[Camera metadata](screenshots/office-parcel-fitting/construction-metadata.json)
records production-calculated pose,50mm,1280×720,High/WebGPU,DOFoff and rain0.82715786;
the numerical pose is an offline calculation, not a browser readback.

Miniature now has the same20second recording and ten decoded samples from the
same checkpoint and held framing. It likewise shows partial glazing followed by
the completed facade/roof, with no obvious fitting discontinuity in the sampled
frames. Normal readiness showed zero geometry queues before recording. See
[Miniature contact sheet](screenshots/office-parcel-fitting/construction-miniature-sheet.png)
and [recording](recordings/office-fitting-construction-miniature.webm). The UI
reported a40.1ms longest frame in one rolling window after recording; this is an
unattributed observation, not an isolated cost measurement or performance pass.

## Matched opening comparison

Seed0/tick58300 office6703 now has matched pre-fitting/candidate captures in both
styles,50mm,High/WebGPU,1280×720,DOFoff,noon preview,rain0.82715786. The same
ordinary save and deterministic Frame review building action were used. Baseline
used a separate Vite load override for the preserved pre-fitting kit and6m office
massing threshold; production files were not reverted. Source hashes and calculated
pose are in [comparison metadata](screenshots/office-parcel-fitting/matched-opening-metadata.json).

Observed: the candidate narrows the body and foundation, restoring a visible gap
to its neighbor while preserving floor count, roof silhouette and street frontage.
The side-clearance gate now enables the already-retained office ribbon glazing,
where the baseline used individual windows. This is a visible consequence of the
fitting correction, not a new facade variant. The surrounding composition matches;
rain animation is not frame-identical. Normal scheduling reached zero queues before
capture. Baseline browser control had one unusually delayed call; that duration
cannot isolate renderer startup cost.

- Anime: [before](screenshots/office-parcel-fitting/open-anime-before.png),
  [after](screenshots/office-parcel-fitting/open-anime-after.png).
- Miniature: [before](screenshots/office-parcel-fitting/open-miniature-before.png),
  [after](screenshots/office-parcel-fitting/open-miniature-after.png).

Outstanding: closer rear assembly review before final retention. Keep the broader release gates open. Sampled frames do not prove
every-frame stability or hardware performance.


Run `node --enable-source-maps scripts/audit-office-parcel-parts.mjs` to regenerate
[the part-level report](office-parcel-parts.json). The script instruments `Kit.add`
after production mass offsets are applied, before material merging, and records
violating component bounds with source call stacks. Repeated components from the
same call stack are grouped with a count and union bounds. It also independently
measures the complete resulting mesh. Tenant-sign canvas material is replaced by
a plain material in Node; geometry is unchanged. Inputs and source are hashed.

| Seed / tick | Offices checked | Violating offices | Width below 6.65 m | Remaining offices: rear AC only | Body itself larger than parcel |
| --- | ---: | ---: | ---: | ---: | ---: |
| 87231 / 83229 | 173 | 30 | 23 | 7 | 11 |
| 0 / 83255 | 173 | 29 | 22 | 7 | 13 |
| 5 / 82901 | 211 | 64 | 54 | 10 | 22 |

All 123 IDs and complete mesh min/max bounds match the independent preceding
office-side-facade audit exactly. Maximum overhang per seed is approximately
0.802 / 0.810 / 0.814 m. This is a local rectangular parcel-envelope check, not a
neighbor collision or whole-city clearance result.

## Established causes

- `partyWallBodyWidth` enforces at least 6 m, including on parcels narrower than
  that body. The foundation adds 0.65 m. On all 99 parcels below 6.65 m the plinth
  crosses the boundary; narrower cases also expose wall, roof, facade and utility
  parts. A plinth-only clamp would leave these other defects intact.
- Every violating component on the remaining 24 offices comes from
  `renderExternalUnits`. Its rear AC grille slats extend 0.6475 m behind the body,
  without checking the actual rear setback. Example: seed87231 office4085 has
  depth7.124498 and body depth6; its backmost mesh point is -3.647500 rather than
  staying within the parcel's -3.562249 boundary. Even office7320 at depth8.027316
  has a small slat overhang. Width alone therefore cannot explain all violations.

## Next bounded correction and acceptance

Fit the affected office bodies and their attached details to the authoritative
parcel, preserving floor counts, use and centered access. Preserve normal-sized
office geometry where it already fits. Fit rear AC assemblies as coherent units
within available setback; do not bury them in walls, clip vertices, or simply
delete the offending geometry. Keep shared-helper impacts on other uses explicit.

Verify actual complete meshes on the same 557 offices, including the previously
passing cases. Add representative narrow and shallow parcel regressions covering
body, foundation, construction stages and entrance clearance. Check ordinary
construction/opening and matched browser views in both styles before retaining a
visual correction. Stop this correction when physical fitting and appearance are
supported; broader facade variants, density changes and extra assets are outside
this slice. Full day-cycle, performance, audio and artistic acceptance remain open.
