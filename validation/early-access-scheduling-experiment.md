# Early interior-access scheduling experiment

The opt-in experiment offers the existing atomic interior-access project at the established-network stage before progressive frontage infill. It leaves starter building work, newly opened-road infill quota, already-usable parcels, active building projects, neighborhood-plan frontage, active road projects, demand/capacity gates, and eligible bridgehead service ahead of the experiment. It uses the existing 160-tick cadence and candidate geometry, reservation, and project lifecycle. The switch and counters exist only in test builds; no production behavior or save schema changed.

Both policies were run from fresh `Engine::new` worlds in the same source, at rate 16 through tick 242,000 for seeds 0 and 87,231. District-policy scoring was off in every row. The ignored Rust test ran 4 worlds in 50.3 seconds total; every world passed `validate`, and a canonical restore followed by a 2,000-tick continuation matched byte-for-byte with the uninterrupted continuation when the non-persisted switch was explicitly restored.

| Seed | Early offer | Open buildings | Housing occupancy / capacity | Active commerce / sites | Envelope proxy (m²) | Floor-area proxy (m²) | Connected ordinary road length | Early admissions | Completed access plans |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 | off | 866 | 7,750 / 7,750 | 344 / 344 | 157,294 | 430,667 | 14,015.8 m | 0 | 1 |
| 0 | on | 868 | 8,114 / 8,114 | 327 / 327 | 160,615 | 470,271 | 14,185.5 m | 3 | 4 |
| 87,231 | off | 851 | 7,945 / 7,945 | 331 / 331 | 157,667 | 463,533 | 13,405.5 m | 0 | 1 |
| 87,231 | on | 834 | 7,996 / 7,996 | 317 / 317 | 158,119 | 467,326 | 13,312.2 m | 3 | 3 |

The treatment added three early access admissions in each seed. Seed 0 ended with two more buildings, 3,321 m² more nominal envelope area, and 39,604 m² more floor-area proxy; it also had 17 fewer active commercial sites. Seed 87,231 ended with 17 fewer buildings, 452 m² more envelope proxy, and 3,793 m² more floor-area proxy; active commerce fell by 14 sites. Both had zero unbuilt parcels. These mixed results do not establish a reliable density gain; actual rendered-kit coverage should be checked from the included snapshots before considering any policy promotion.

The four authoritative native saves and snapshots are `early-access-seed-{0,87231}-enabled-{false,true}-{save,snapshot}.json`. Their SHA-256 values are:

- Seed 0, baseline save `fb74d841dec5612dbd3e9ae418711d375fd212e441c6f0e094c6bde38c87590b`; treatment save `fec2ca0b34b3e6b14b23f11ed799be3dc5a10e60dbfb35ffc4d41609158c7e0`.
- Seed 87,231, baseline save `e3ea0b5c097d89c31621a20f4625dab1e3f56bef8f4f9bb14b217ad3d0df707d`; treatment save `f19ae1c170d8b2391b414e2be57e610a5780b8f5a1833caaba27a2ad97ea4e26`.

The machine-readable metrics are in [early-access-scheduling-experiment.json](early-access-scheduling-experiment.json). Envelope and floor-area figures are nominal building-envelope proxies; they are not rendered ground coverage. The trial changes later RNG and project timing through real construction, so two seeds are not enough to claim a general improvement.
