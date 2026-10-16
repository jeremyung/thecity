# Camera and Scene Direction

The camera reveals growth while preserving the close, miniature scale that makes ordinary activity enjoyable. Manual exploration always takes priority.

## Camera modes

Observe holds or slowly moves through a scene. Explore gives direct control. Follow tracks a selected subject with a stable offset. Overview frames the developed footprint. Lighting preview retains the existing mode while changing only lighting.

Manual input cancels automatic motion immediately and smoothly. Do not reclaim control after a timeout; provide Watch to resume. Orbit and zoom stay inside safe camera bounds, above terrain and outside buildings. Keyboard and touch behavior have equivalent outcomes.

## Finite scene anchors

The director maintains a bounded set of anchors: founding shop, river crossing, station, active construction, neighborhood street, waterfront, and broad skyline. Derive anchors from real world entities and pedestrian-accessible locations, not invented narrative events.

Store a persistent identity, geographic extent, available camera poses, visual importance, activity summary, last visit, and validity. Refresh anchors when construction opens or a district changes. Keep at most 32 active candidates initially.

## Showing expansion

Track the connected developed footprint and a smoothed extent. Do not widen the camera for a distant project marker or one empty road segment.

When sustained footprint growth exceeds about 10–15% since the last framing update, schedule a broader reveal. Smoothly pull back or rise over roughly 20–40 seconds. Use hysteresis and a minimum interval between reveals to avoid repeated reframing.

Limit zoom-out by readability: if street activity becomes too small, hold the scale and introduce another scene. A finite world overview is occasional orientation, not the permanent endpoint of the camera.

Typical progression:

1. Start close to the founding shop and first construction site.
2. Ease back to reveal a newly opened lane and homes.
3. Follow a delivery along that lane into a new neighborhood.
4. Hold a station scene as its first service arrives.
5. Briefly reveal two connected centers and the river crossing.
6. Return to an intimate street scene at dusk.

This is an example of event-driven direction, not a fixed story every seed repeats.

## Scene selection

Rank valid anchors by visible activity, meaningful change, composition, time-of-day suitability, and time since last visit. Penalize repetition, busy camera travel, and scenes already observed. Cosmetic selection randomness is independent of authoritative RNG.

Hold a scene for at least 30 seconds initially; quieter scenes may last 60–120 seconds. Avoid cuts simply because an event occurs. Prefer a subtle reframing when the new subject is nearby. For distant scenes, use a short dissolve or intentional slow journey. Do not fly through buildings to reach a target.

At night, favor stations, illuminated streets, and waterfront reflections. Dawn favors emerging activity and quieter roads. Rain favors shelter, crossings, and reflective surfaces. These are composition preferences, not changes to the world.

## Follow behavior

Follow a train with look-ahead along the track and restrained camera acceleration. Follow a resident at neighborhood scale rather than face level. When the subject enters a building, settle on the destination, show the activity on request, and release follow gracefully. If a target disappears, hold the last valid scene and explain its current state without snapping to the origin.

## Quality and comfort

No continuous camera roll, oscillating focus, dramatic acceleration, or rapid cuts. Focus follows a stable subject with damping. Reduced motion uses static holds and optional user-initiated scene changes. Audio listener movement is smoothed independently during cuts to avoid loud abrupt transitions.

## Acceptance criteria

- A growing district remains framed without continuous tiny zoom adjustments.
- New neighborhoods become visible without making the original one disappear from history.
- A full city still offers readable close scenes.
- User interaction interrupts any automatic move on the next frame.
- Camera mode and style never alter simulation hashes.

## Photographic lens model

Camera position, target, and lens are distinct viewer state. Dolly changes camera distance and perspective relationships; focal length changes angular framing. A style switch preserves all three. Lens choices must not affect authoritative simulation state or RNG.

Developer visual review includes a held waterfront preset: arc-length river
midpoint target, fixed camera offset, normal bounds/collision recovery, and
Explore hold with no automatic drift. It preserves the selected lens and world
state. This explicit diagnostic cut is not part of normal Watch transitions.
See [repeatability evidence](../validation/waterfront-review-camera.md).

The initial physical lens library uses a 24 mm-high sensor with 35 mm environmental, 50 mm neighborhood, and 85 mm detail presets. Vertical field of view is derived from sensor height and focal length; it is not an unrelated artistic constant. Aperture is expressed as f-number. Focus distances are in scene metres and optical dimensions in millimetres. The thin-lens circle of confusion is converted to output pixels and capped to a bounded kernel budget.

A physical lens viewing a full-size city hundreds of metres away produces little defocus. The miniature treatment therefore needs an explicit optical model scale or a separately named artistic tilt-focus treatment, rather than silently misrepresenting metres as millimetres. A tilted focus plane is distinct from ordinary depth blur. Before promotion, compare a fixed city/camera/time across lens settings, show foreground/subject/background focus behavior, preserve manual camera priority, and verify stable anti-aliased edges during motion in both render backends.

Follow lifetime contract: a missing movement-only subject holds its last desired
pose for a four-second grace period, then returns to Watch. Reappearance cancels
the grace period. Structural indoor/onboard positions remain valid fallbacks;
expired inspection results clear the subject panel.

Dated implementation notes for this spec: [validation/spec-notes/06-camera.md](../validation/spec-notes/06-camera.md).
