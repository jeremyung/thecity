# Checkpoint notes moved from specs/02-world-growth.md

Dated candidate, checkpoint and status notes, moved verbatim on 2026-10-16 so the
spec describes current design only. Newest decisions win over older notes.

Height-renewal candidate (2026-10-12): mature unreferenced station offices can
close, undergo demolition and rebuild taller in place. Fresh WASM growth and
in-progress replay are verified for one seed; a four-seed, thirty-day sampled
audit covers repeated renewal and final live/restore continuation. Housing
relocation, lot assembly, skyline composition and visual acceptance remain open. See
[candidate evidence](../validation/height-redevelopment.md).

Pocket parks (2026-10-14): every quarter day, a master-plan city may keep one still-vacant ordinary lot (80–520 m², connected, not a station-tower site) as protected open space. The lot needs at least eight occupied homes within 70 m and must stand at least 120 m from any other park. The city holds at most one park per 30 occupied homes. Only pocket parks count toward this cap and the spacing; shrine and plaza lots do not. Protected lots are never developed. The choice is deterministic, draws no random numbers, and records a `ParkDedicated` event. Seed 3 at development rate 60 grows to 11 parks by maturity. The renderer draws each one as a fenced neighborhood playground.

Station plaza and shrines (2026-10-14): when master-plan lots are first released, up to two street-facing lots within 45 m of the planned station stay open as its plaza. Each residential neighborhood keeps one ordinary lot of 150–600 m² for a shrine. A shrine stands at least 150 m from the station and 260 m from other shrines, with at most six per city. These lots are protected and record `PlazaReserved` or `ShrineReserved`. Parcels carry an optional `open_space` kind (`park`, `shrine` or `plaza`). Parks from earlier saves have no kind and still render as parks. The seed 87231 review city keeps 2 plaza lots and 3 shrines.

## Implementation status (2026-10-11)

The master plan, its growth realization, narrow streets, paved blocks, party-wall rendering, the elevated rail viaduct and the expressway are implemented. Three mature seeds measure 49.6–51.6% rendered coverage in compact crops and 39.9–42.2% in station crops, against 28.9% and 20.1% for the legacy planner. Evidence, captures and open items are in the [master-plan review](../validation/masterplan-review/README.md). Not yet implemented: lot assembly and height renewal, multi-station rail service, bus routes on arterials, distant low-rise LOD, and removal of the legacy planner.

Elevated-route refinement: before deriving cubic controls, three fixed lateral
fairing passes smooth the independent elevated nodes along the chosen arterial
corridors. Longitudinal coordinates and outer interior anchors remain fixed;
street nodes remain unchanged. Thus elevated routes need not pass exactly over
each noisy street junction. Clearance and lot admission use the refined route,
and separate deck curves cross spatially without requiring coincident nodes.
See [measured validation](../validation/expressway-fairing.md).

Regional expressway boundary tails continue the approaching interior bearing
to the finite boundary; do not reset short end segments to cardinal axes.
Curve admission and traffic use the resulting authoritative controls. A fixed
seed corpus checks bounded endpoints, tangent continuity and sampled fold risk.
