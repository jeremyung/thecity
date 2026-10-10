# Sound and Music

Sound gives the city a sense of place. Spatial everyday activity sits beneath an original or licensed indie lo-fi chillhop score. Music and city audio can be controlled separately.

## Layers

1. Distant city bed: low-level traffic, wind, rail, and neighborhood murmur.
2. Local environment: river, rain, trees, sheltered alleys, station surroundings.
3. Activity emitters: train approach, vehicle passage, footsteps, shop doors, crossing signals, construction.
4. Music: warm keys, restrained percussion, bass, and ambient texture in compatible stems.

Use Web Audio routing for buses, spatial emitters, filtering, fades, and scheduled music. Start audio only after a viewer interaction. Honor mute and saved levels.

## Spatial policy

Limit active spatial sources, initially 32, using distance, importance, and audibility. Aggregate distant crowds and footsteps rather than allocating an emitter for each traveler. Construction softens with distance. Avoid constant prominent horns and announcements.

Camera zoom blends neighborhood and city beds. A visual camera cut crossfades listener context over approximately one second. Sound should preserve calm even when the camera selects a busy scene.

## Synchronization

Movement snapshots drive continuous train and vehicle emitters. Simulation events trigger arrivals, openings, and construction stage cues. Deduplicate event IDs and ignore stale events after loading. Do not queue every missed footstep when fast-forwarding; aggregate activity.

Pause freezes action-linked sounds while optional quiet ambience and music remain configurable. Hidden tabs fade and suspend audio. Resume against the current world, not a backlog of sounds.

## Musical direction

Use a shared tempo and musical key across stems. Schedule changes on musical boundaries using the audio clock, not render frames. Dawn can introduce sparse keys; day adds soft percussion; dusk warms the mix; late night removes layers. Weather changes texture gradually.

Music does not increase intensity with population as though rewarding progress. Mature quiet scenes remain quiet. Style selection may suggest a mix preset, but must not restart the track or override viewer audio preferences.

## Assets and rights

Maintain an asset manifest with source, license, attribution requirements, duration, loop points, and normalization notes. Japanese speech and signage should be authored or reviewed where meaningful; avoid fabricated prominent announcements. Reusable nonverbal ambience is sufficient for the first release.

## Validation

Verify smooth loops, no clicks on fades, no clipping at maximum allowed settings, sensible headphone spatialization, mobile resume behavior, and ambience-only operation. Sound must reflect actual nearby activity.

[Web Audio API documentation](https://developer.mozilla.org/en-US/docs/Web/API/Web_Audio_API)
