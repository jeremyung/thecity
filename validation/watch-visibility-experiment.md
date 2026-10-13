# Watch street visibility experiments — 2026-10-12

Current decision: first steep-view experiment rejected; second lower-angle
alternate-street candidate retained provisionally. The rejection below applies
to the first experiment only. Latest follow-up evidence appears at the end.

Previous goal turn made progress by separating Follow subject focus from camera
look-ahead. This turn tested the next observed deficiency: ordinary Neighborhood
street Watch shots may have large foreground buildings obscuring the named subject.

Source evidence: district anchors use building-position centroids and a fixed
shallow direction; camera travel clearance does not imply subject visibility. The
candidate projected each district centroid onto a nearby open local/arterial road,
kept 12 m from its ends, and tested three street points against building envelopes.
A bounded 24-pose selector tried alternate yaw and increasingly steep elevations.
The chosen shot was cached across unchanged geometry and manual Explore retained
priority. Existing no-road fixtures retained their original neighborhood fallback.

Luna supplied the pure selector and integration regression; root inspected and
integrated it. 60 focused tests and TypeScript passed in the candidate. Tests proved
geometric visibility, bounded search, safe destination and stable pose, but did not
prove the required visual composition.

Browser: imported seed 0 / tick 9250 / 260 buildings while paused through normal
scheduler. Miniature, 50 mm, DOF on, High, WebGPU, 847×764, live lighting 15:15,
checkpoint weather unchanged. Initial Neighborhood street selection produced a
steep overhead roof/street-canyon view. The street was exposed but the layered,
photographic composition was poorer than the intended concept direction. This
was rejected. `screenshots/watch-visibility/rejected-steep-street.png` preserves
the failure witness; it is not a matched old/new camera pair. An earlier HMR cycle
showed a graphics startup timeout without captured console errors; subsequent HMR
started normally, so startup reliability is not claimed by this test.

All candidate source and its exclusive tests were removed from production; the
experiment is preserved in `watch-visibility-rejected.patch`. The prior Follow
focus correction remains intact. After restoration, 58 camera/review/focus tests
and TypeScript pass; diff whitespace passes. No unrelated dirty work was reverted.

## Next bounded approach

Do not promote a steep angle merely because its rays are clear. Limit the artistic
angle range first, then consider alternate nearby street subjects with real activity.
If no pleasing view exists, retain or choose a different authored anchor. Keep the
current hold stable across ordinary centroid changes; re-evaluate at deliberate
scene changes or loss of the subject. Compare actual compositions before adding
broader caching or heuristics. One unsuccessful approach has been tried here; a
second unsuccessful attempt requires reassessment rather than more parameter stacking.

This evidence changes the next implementation choice; Watch framing, motion,
both-style review and locked-concept fidelity remain incomplete.


## Lower-angle alternate-street candidate — 2026-10-12

The second experiment is retained as a candidate after a materially better browser
composition. Instead of escalating to steep overhead views, it tries up to four
nearby open local/arterial streets and at most 18 poses per street. Slopes are
bounded at .65 (about 33 degrees); boundary clamping cannot silently steepen an
accepted pose beyond that. Each candidate tests the street center and +/-12 m
against building envelopes, separately checking the camera destination. Existing
collision-safe travel still controls transitions. Clear views keep their original
preferred angle; selection is deterministic and cached across unchanged geometry.
The street point stays pinned while neighboring building centroids change.

Root integrated the selector and alternatives; Luna provided selector tests,
integration coverage and a lifecycle review. The review caught road-closure handling:
cache now records the actually selected road, not only the nearest candidate. If it
closes, Watch transitions to a rebuilt district fallback even if there is no new
eligible street. Tests cover that fallback and manual Explore priority.

Browser study uses the same seed 0 / tick 9250 / 260-building paused checkpoint as
the rejected trial, Miniature, 50 mm, High, DOF on, live 15:15 lighting, checkpoint
weather, 847x764, WebGPU, normal import scheduler. The selected shot exposes a
street/rail corridor as a diagonal, with nearby and distant facades forming layers.
It is more readable and photographic than the steep roof canyon. Root switched to
Explore to hold it for `screenshots/watch-visibility/low-angle-candidate-on.png`.
This is a composition study, not a pixel-identical camera comparison or a claim of
locked-concept fidelity. Later changes only corrected closure handling, boundary
angle rejection and internal visibility status; final both-style/motion checks
remain pending. HMR again briefly showed the startup timeout UI, then recovered;
startup reliability remains a separate unresolved observation.

76 focused camera/selector/review/focus/lens tests and TypeScript pass. Production
build passes with the existing bundle-size warning. If no candidate is visible,
the current implementation retains a generic fallback with explicit internal
visible=false; that fallback has no visual acceptance. Street samples do not model
tree, station-canopy or bridge occlusion. CPU cost, ordinary growth motion, lens/
aspect changes, nighttime and Anime review must be checked before promotion.


CPU follow-up: root inspected and reran `scripts/audit-watch-shot-cost.mjs`.
Current `watch-shot-cost.json` reports Node-only seed0/tick9250 measurements on
Apple M2 Max: anchor build maximum 0.914 ms; 75 cold district shot selections
p50 0.790 ms, p95 3.031 ms, maximum 5.054 ms. Twelve of fifteen district anchors
found a clear street candidate; three did not. This supports a bounded candidate,
not browser frame-time acceptance. The additional old growth-575 checkpoint fails
current save validation (`traveler 3011 destination connector does not meet its
route endpoint`), so it was explicitly excluded; no mature-city coverage is claimed.

## Style and sampled transition follow-up

The held street/rail composition was captured in both styles with DOF disabled:
`screenshots/watch-visibility/held-miniature-dof-off.png` and
`screenshots/watch-visibility/held-anime-dof-off.png`. Same paused seed0/tick9250,
260 buildings, 50mm, High, WebGPU, live15:15 and unchanged checkpoint weather;
normal scheduler queues settled to zero. The diagonal corridor and layered facades
remain readable in both styles. Anime has different edge/surface treatment even
without blur. Dark shaded facades and sparse ground activity remain perceptual
gaps; neither image establishes locked-concept fidelity.

`screenshots/watch-visibility/anime-watch-transition.webm` records a return from
Explore to Watch with DOF enabled and the simulation paused. Reviewed samples at
1, 10 and 19 seconds show progression from the street/rail corridor toward a
readable arterial view, with no obvious clipping in those samples. Video metadata
after seeking reports 19.942 seconds at 1355x1217. `transition-19s.png` preserves
the final sample; `motion-review.html` supports manual seeking. This is sampled
transition evidence, not an all-frame motion, moving-agent or growth acceptance.

Next visual priority is shaded-facade readability. The visible sky mesh does not
provide environment illumination; daytime hemisphere fill falls to0.5. A bounded
trial raised it to1.0 while retaining nighttime1.05 and all other lighting values.
The comparison checkpoint was restored through the normal importer and all four
render queues reached zero, but the browser screenshot surface returned zero
width even with an explicit baseline viewport. No candidate image was obtained.
The single lighting edit was reverted; no artistic benefit is claimed and no
unreviewed lighting adjustment remains. Retry a matched comparison when captures
work. Nighttime, alternate lenses/aspects, generic fallback views, and ordinary
growth motion remain open camera gates.
