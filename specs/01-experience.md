# Experience and Scope

The city is a living audiovisual place that rewards attention without requiring action. Success means the viewer enjoys leaving it running and recognizes streets that have changed since they last looked.

## Viewer role

The viewer may explore, inspect, follow, pause, adjust time, select a visual style, and start or resume a world. There are no scores, objectives, unlocks, management obligations, compulsory notifications, or failure screens. No road painting or building placement in the initial scope.

The default experience is a gently directed camera with a minimal control strip. Inspectors and statistics open on request. The world remains enjoyable with all overlays hidden.

## World lifecycle

1. Load a seed and generate finite geography and potential transport corridors.
2. Establish one external gateway and a sparse founding settlement.
3. Show early deliveries, road work, and building construction.
4. Expand through connected neighborhoods and transit anchors.
5. Reach a mature city where infill, renovation, businesses, and routines continue changing.

Generation should reveal the world progressively without exposing half-valid terrain or disconnected routes. Early activity must appear before a long wait for citywide detail. Proposed targets: visible construction within 30 seconds after world readiness, recognizable settlement within two minutes, lively neighborhood within ten minutes at default development speed.

## Controls

| Control | Meaning |
| --- | --- |
| Camera pan, orbit, zoom | Explore within world bounds |
| Watch mode | Resume autonomous scene direction |
| Follow | Track a person, train, vehicle, or construction project |
| Pause | Stop simulation and associated action; retain camera interaction |
| Simulation speed | Advance all authoritative clocks at a shared multiplier |
| Development pace | Separately adjust long-term growth rate |
| Lighting preview | Temporarily preview a sun angle without changing history or schedules |
| Style | Switch rendering treatment without changing the world |
| Audio | Independent music and city levels; ambience-only option |
| World | Show seed, save, resume, or start a new city |

Show lighting preview as a distinct mode; returning to live restores authoritative time. Do not imply historical time travel. Preserve the current world before replacing it.

## Interface states

- Loading: communicate generation or asset progress using real milestones.
- First visit: city visible immediately after readiness; one brief pointer to exploration controls.
- Observing: unobtrusive controls and optional city clock.
- Inspecting: concise place history derived from real events, with location and occupancy.
- Following: identify the subject and provide an obvious exit.
- Paused: persistent but quiet state indicator.
- Hidden tab: pause and checkpoint; resume without unbounded catch-up.
- Recoverable graphics failure: restore from the latest snapshot and lower quality if needed.
- Unsupported graphics: clear explanation and retry path; no broken canvas.
- Mature city: continue normal observation; do not announce completion as an ending.

## Accessibility and devices

Provide keyboard camera and controls, visible focus, text inspectors outside the canvas, separate audio controls, and adjustable contrast. Reduced motion disables automatic camera travel and uses fixed scene holds. It may reduce environmental motion separately without changing simulation outcomes. Depth of field can be disabled.

Desktop is the first quality target. Mobile retains exploration, pause, time, style, and audio controls through touch controls and compact sheets; quality scales with the device.

## First public scope

A small complete landscape with procedural roads and parcels, visible infrastructure and building construction, purposeful trips, rail service, a complete daily cycle, dynamic river water, sound, local saves, and Last Train, First Light rendering. Little Tokyo is the second renderer. Painterly and ink remain later options.

Defer multiplayer, shared regional economies, interiors, disasters, management tools, historical replay UI, and citywide physical fluid simulation.
