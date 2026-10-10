# Bus service implementation contract

This plan adds a small, working bus service to the simulation. It uses the existing road graph, resident routines, walking connectors, deterministic save state, and movement stream. A bus is transportation infrastructure with passengers and destinations, not a decorative vehicle loop.

## Service shape and bounds

- Begin with one line and one bus. Permit at most two buses on the line, 24 seats per bus, and 48 waiting residents across all stops. Keep the line to at most five stops and 128 road nodes. These limits bound route searches, queue work, rendering, and save size.
- Bring the line online only when the station is usable, at least six connected ordinary road segments are open, and three or more developed building catchments can reach proposed stops. Stops must sit on open, connected, non-rail road nodes beside active occupied homes or workplaces.
- Choose stops deterministically from the seeded world: the station, the founding market/cafe area, and up to three separated developed neighborhoods. Prefer useful demand and road connectivity; use seeded tie-breaking. Build one continuous route through the selected nodes, with a return direction on that corridor. Do not draw a line across disconnected nodes.
- Run from 05:00 to 01:00. A bus already carrying passengers finishes its current run during quiet hours, drops them at a valid stop, then waits until service resumes. Departures use a 12–18 simulation-minute headway for one bus. Add the second bus only when a deterministic demand threshold is reached.
- Keep route and stop selection bounded and event-driven: rebuild only after meaningful road/building changes, and only when a bus is at a terminal with no passengers. Do not teleport a moving bus or reroute riders mid-trip.

## Persistent simulation state

Add serde-defaulted state so existing saved worlds load without migration failures:

- A bus line stores its stable ID, ordered stop node IDs, bounded road-node route, graph revision used to build it, and service status.
- Each bus stores stable ID, line ID, absolute distance along the route, direction, dwell ticks, and a bounded ordered list of resident IDs aboard. Progress is distance from the first route node, independent of direction; reversal changes direction at a terminal without changing position.
- Residents add optional bus journey fields: bus ID, origin stop, destination stop, and queue slot. Use explicit transfer states for walking-to-stop, waiting, riding, and walking-from-stop. Legacy saves default these fields to no bus trip.
- Snapshot bus views expose route/stops, absolute progress, direction, dwell state, passenger count, capacity, and waiting count. Resident views expose optional bus ID and stop while transferring. Keep current snake_case JSON conventions. Add movement kind 3 for buses; retain kinds 0/1/2 for pedestrians/cars/trains.
- While aboard, a resident is represented by the bus passenger list and is absent from pedestrian/car traveler movements. Waiting and walking residents remain visible as pedestrians. Bus vehicles are reported separately from active human traveler counts so existing population/activity statistics retain their meaning.
- Include all new counters, route state, dwell, queues, and passenger assignments in save/restore. A save restored during any transfer phase must replay identically.

## Resident journey lifecycle

Use the bus for a subset of ordinary resident work trips whose home and workplace both have walking access to served stops and whose stop pair has service in the required direction. A practical initial catchment is a bounded walking route of at most 75 metres from each building door to its stop. Rail commuters remain on the current rail-transfer flow; bus-to-rail transfers can be a later extension.

1. At the resident's deterministic departure time, route from the real home door to the nearest eligible stop using the existing sidewalk/road access connectors. If no valid stop pair or route exists, fall back to the current walking or car mode.
2. Place the resident in a bounded stop queue at a physical platform position adjacent to the road. Queue slots are stable and spread along the curb; the resident stays visible and waits for a bus serving the destination in the correct direction.
3. During stop dwell, board in stable resident-ID order until capacity is full. A full bus leaves remaining residents queued for the next service. Boarding changes the resident to riding state and adds the ID to the bus exactly once.
4. Move the bus along the active road route with the existing vehicle lane convention, signals, following-distance rules, and speed limits. Stop at each scheduled stop, dwell for a bounded duration, and alight only riders whose destination is that stop.
5. Walk from the destination stop to the actual workplace door. Preserve the resident identity, trip purpose, and journey history through walking, waiting, riding, alighting, and arrival. Schedule the return journey through the same lifecycle.

Bound each tick's boarding, alighting, route-search, and queue operations. Waiting queues have a hard capacity; overflow residents use the existing walking/car fallback rather than becoming permanently invisible or unbounded. Never create a journey unless both endpoints and every leg are connected and reachable.

## Rendering, camera, audio, and UI integration

- Renderer: interpret movement kind 3 as a bus; render it on the left-hand road lane with headlights, emissive destination display, windows, wheels, shadows, FSAA, and the selected visual treatment. Add small bus-stop poles or shelters only at active stops. Show doors/indoor light while dwelling. Keep the model and stop details consistent across painterly/inky/anime shader styles.
- Camera and selection: assign the bus a stable selectable entity ID and support follow mode across turns, stops, and terminal dwell. Keep camera motion smooth when the bus pauses. If a selected resident boards, follow the bus through the resident's bus ID and return to the resident after alighting. Waiting and walking residents remain individually followable.
- UI/types: add BusView and movement kind 3 to the TypeScript contracts, with optional fields for older fixtures. The inspector should show line, next stop, direction, load/capacity, dwell, and waiting queue. Resident inspection should show the current transfer phase and bus. An optional observation overlay may show stop locations and the line; it must not turn the simulation into a route-planning game.
- Audio: derive a low-volume spatial engine/road wash from bus position and speed, attenuated by distance and occlusion. Emit at most one soft brake/door cue per stop dwell, with a per-bus cooldown. Avoid a sound per passenger. Service-start and boarding cues should be subtle and optional. Continue to use the existing weather and wet-road ambience.
- Fixtures: regenerate public WASM bindings and mature-world fixtures only after the serialized shape is stable. Keep compatibility with saves and tests missing all bus fields.

## Validation contract

Native and WASM tests should establish behavior, bounds, determinism, and visible movement:

- No service before its infrastructure/demand prerequisites; once eligible, stops and every route edge are open, connected, non-rail, and bounded. Rebuilding a stale line occurs only at a safe terminal.
- Journey lifecycle test: a named resident walks from a door to a stop, visibly queues, boards, rides, alights at the correct stop, walks to the workplace door, and later returns. Identity/history remain continuous and no duplicate pedestrian exists aboard.
- Capacity and queue tests: a bus never exceeds 24 riders; remaining residents wait for the next bus; global waiting never exceeds 48; overflow follows the explicit fallback.
- Movement tests: buses use the correct lane and heading, stop at served nodes, honor dwell and traffic signals/following gaps, never jump on reversal or route refresh, and do not board/alight at an unserved or wrong-direction stop.
- Quiet-hours test: no new departure from 01:00–05:00, an in-progress trip completes safely, and queued riders are not stranded indefinitely when service resumes.
- Persistence tests: save/restore/replay byte-identically while a resident is walking to a stop, queued, aboard a moving bus, during dwell, and walking the final leg. A legacy save with no bus keys loads unchanged and deterministically.
- Fallback tests: disconnected catchments, unavailable buses, wrong-direction service, and missing destinations select the established walk/car modes without ghost riders.
- Seed corpus: test at least 12 seeds for bounded bus/line/queue counts, no invalid IDs, and deterministic snapshots. A long accelerated run should show recurring journeys without memory growth proportional to elapsed ticks. Record actual route/stop/passenger counts and a measured profile, rather than asserting a fixed city size.

## Roadside frontage observation

The current frontage geometry intentionally leaves green setbacks and side gaps. Parcel centers sit about 15 metres from the road centerline. With an ordinary road half-width near 4.5 metres and a building depth near 12 metres, the facade lies roughly 9 metres from the centerline, leaving about 4.5 metres of verge between road edge and facade. Arterials are wider, so their verge is closer to 2 metres. Along a frontage, lots about 14 metres wide on roughly 19-metre slots leave around 5 metres between building footprints. These spaces create room for sidewalks, planting, fences, steps, and tiny frontage details, but broad empty lawns can dominate before infill unlocks.

The door/sidewalk connectors already represent physical access; the visible setback itself does not mean pedestrians are routed to a distant junction. If the lawn reads too broad, first render the verge as an intentional sidewalk/planting strip and add authored frontage detail. Only reduce the 15-metre setback after checking parcel collisions, river-bank clearance, door paths, and roadside placement together. Do not solve visual gaps by adding lots every tick or bypassing the existing bounded infill rules.
