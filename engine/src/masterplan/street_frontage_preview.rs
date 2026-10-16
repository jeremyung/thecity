//! Isolated bounded street-placement candidate and offline preview. Not a legal-site
//! admission test: elevated/rail reservations and further street recursion follow.
use super::*;
use serde::Serialize;

const PREVIEW_SEEDS: [u32; 3] = [17, 53, 101];
const PROPOSED_STREET: usize = usize::MAX;

#[derive(Debug, Serialize, PartialEq)]
struct Preview {
    inset_area: f32,
    direct_footprint_by_seed: [f32; 3],
    rear_footprint_by_seed: [f32; 3],
    direct_lots_by_seed: [usize; 3],
}

fn evaluate(polygons: &[Poly], district: District, kind: &dyn Fn(usize) -> RoadKind) -> Preview {
    let inner: Vec<_> = polygons
        .iter()
        .cloned()
        .flat_map(convex_pieces)
        .map(|p| {
            inset(&p, |label| {
                label.map_or(0.0, |line| kind(line).lot_setback())
            })
        })
        .collect();
    let mut result = Preview {
        inset_area: inner.iter().map(area).sum(),
        direct_footprint_by_seed: [0.0; 3],
        rear_footprint_by_seed: [0.0; 3],
        direct_lots_by_seed: [0; 3],
    };
    for (index, seed) in PREVIEW_SEEDS.into_iter().enumerate() {
        // Every alternative begins with the same private stream. Never consumes
        // the generator stream or clones whatever state preceding cuts left it in.
        let mut rng = Rng::new(seed);
        let mut lots = Vec::new();
        #[cfg(test)]
        let mut losses = Vec::new();
        for (block, p) in inner.iter().enumerate() {
            if p.len() < 3 {
                continue;
            }
            partition_block(
                p,
                district,
                block as u32,
                &|line| kind(line) == RoadKind::Arterial,
                &mut rng,
                &mut lots,
                false,
                #[cfg(test)]
                &mut losses,
            );
        }
        for p in lots {
            let footprint = p.lot.width * p.lot.depth;
            if p.lot.flag {
                result.rear_footprint_by_seed[index] += footprint;
            } else {
                result.direct_footprint_by_seed[index] += footprint;
                result.direct_lots_by_seed[index] += 1;
            }
        }
    }
    result
}

struct Alternative {
    attempt: usize,
    hits: Vec<(P, Option<usize>)>,
    children: Vec<Poly>,
    preview: Preview,
}

fn alternatives(
    builder: &Builder,
    parent: &Poly,
    district: District,
    proposed_kind: RoadKind,
    center: P,
    base_n: P,
    extent: f32,
    min_piece: f32,
    jitter: f32,
) -> Vec<Alternative> {
    let mut proposals = Vec::new();
    let kind = |line: usize| {
        if line == PROPOSED_STREET {
            proposed_kind
        } else {
            builder.lines[line].kind
        }
    };
    let mut rng = Rng::new(719);
    for attempt in 0..8 {
        let tilt = rng.range(-jitter, jitter);
        let n = base_n.add(P::new(-base_n.z, base_n.x).scale(tilt)).unit();
        let room = (0.5 - min_piece / extent).clamp(0.0, 0.16);
        let origin = center.add(base_n.scale(rng.range(-room, room) * extent));
        let mut hits = Vec::new();
        for i in 0..parent.len() {
            let (p, label) = parent[i];
            let q = parent[(i + 1) % parent.len()].0;
            let sp = p.sub(origin).dot(n);
            let sq = q.sub(origin).dot(n);
            if (sp >= 0.0) != (sq >= 0.0) {
                hits.push((p.lerp(q, sp / (sp - sq)), label));
            }
        }
        if hits.len() != 2 {
            continue;
        }
        let clear = hits.iter().all(|(p, label)| {
            label.is_some_and(|line| builder.nearest_junction(line, *p) > 14.0)
                && parent.iter().enumerate().all(|(i, v)| {
                    if v.1 == parent[(i + parent.len() - 1) % parent.len()].1
                        && v.1.is_some_and(|l| !builder.lines[l].path.is_empty())
                    {
                        return true;
                    }
                    v.0.sub(*p).len() > 14.0
                })
        });
        if !clear {
            continue;
        }
        let children = vec![
            clip(parent, origin, n, Some(PROPOSED_STREET)),
            clip(parent, origin, n.scale(-1.0), Some(PROPOSED_STREET)),
        ];
        if children
            .iter()
            .any(|p| p.len() < 3 || area(p).abs() < 500.0)
        {
            continue;
        }
        let preview = evaluate(&children, district, &kind);
        proposals.push(Alternative {
            attempt,
            hits,
            children,
            preview,
        });
    }
    proposals
}
/// Preserve the same host streets; rank by worst improvement across all probes.
/// Return None to retain the baseline. No evaluation consumes generator RNG.
#[cfg(feature = "street-placement-candidate")]
pub(super) fn choose(
    builder: &Builder,
    parent: &Poly,
    a: &Poly,
    b: &Poly,
    hosts: [usize; 2],
    district: District,
    proposed_kind: RoadKind,
    center: P,
    base_n: P,
    extent: f32,
    min_piece: f32,
    jitter: f32,
) -> Option<(Poly, Poly, Vec<(P, Option<usize>)>)> {
    // Unlabelled parent edges are not street access; leave these cases unchanged.
    if parent.iter().any(|(_, label)| label.is_none()) {
        return None;
    }
    let kind = |line: usize| {
        if line == PROPOSED_STREET {
            proposed_kind
        } else {
            builder.lines[line].kind
        }
    };
    let baseline: Vec<_> = [a, b]
        .into_iter()
        .map(|p| {
            p.iter()
                .map(|(p, l)| (*p, l.or(Some(PROPOSED_STREET))))
                .collect()
        })
        .collect();
    let baseline = evaluate(&baseline, district, &kind);
    let mut selected = None;
    let mut best_gain = 0.0;
    for proposal in alternatives(
        builder,
        parent,
        district,
        proposed_kind,
        center,
        base_n,
        extent,
        min_piece,
        jitter,
    ) {
        let candidate_hosts = [proposal.hits[0].1.unwrap(), proposal.hits[1].1.unwrap()];
        if candidate_hosts != hosts && candidate_hosts != [hosts[1], hosts[0]] {
            continue;
        }
        let gain = proposal
            .preview
            .direct_footprint_by_seed
            .iter()
            .zip(baseline.direct_footprint_by_seed)
            .map(|(new, old)| *new - old)
            .fold(f32::INFINITY, f32::min);
        if gain > best_gain {
            best_gain = gain;
            selected = Some(proposal);
        }
    }
    selected.map(|proposal| {
        let mut pieces = proposal.children.into_iter().map(|p| {
            p.into_iter()
                .map(|(p, label)| {
                    (
                        p,
                        if label == Some(PROPOSED_STREET) {
                            None
                        } else {
                            label
                        },
                    )
                })
                .collect()
        });
        (
            pieces.next().unwrap(),
            pieces.next().unwrap(),
            proposal.hits,
        )
    })
}

#[cfg(test)]
pub(super) fn trace(
    builder: &Builder,
    parent: &Poly,
    a: &Poly,
    b: &Poly,
    district: District,
    proposed_kind: RoadKind,
    depth: u32,
    center: P,
    base_n: P,
    extent: f32,
    min_piece: f32,
    jitter: f32,
) {
    let kind = |line: usize| {
        if line == PROPOSED_STREET {
            proposed_kind
        } else {
            builder.lines[line].kind
        }
    };
    let children: Vec<_> = [a, b]
        .into_iter()
        .map(|p| {
            p.iter()
                .map(|(point, label)| (*point, label.or(Some(PROPOSED_STREET))))
                .collect()
        })
        .collect();
    let original = evaluate(&[parent.clone()], district, &kind);
    let split = evaluate(&children, district, &kind);
    let proposals: Vec<_> = alternatives(builder,parent,district,proposed_kind,center,base_n,extent,min_piece,jitter)
        .into_iter().map(|a|serde_json::json!({"attempt":a.attempt,"preview":a.preview,
            "improvesAllProbeSeeds":a.preview.direct_footprint_by_seed.iter().zip(split.direct_footprint_by_seed).all(|(new,old)|*new>old),
            "endpoints":a.hits.iter().map(|(p,_)|[p.x,p.z]).collect::<Vec<_>>(),
            "children":a.children.iter().map(|p|p.iter().map(|(p,_)|[p.x,p.z]).collect::<Vec<_>>()).collect::<Vec<_>>() })).collect();
    eprintln!(
        "STREET_FRONTAGE_PREVIEW {}",
        serde_json::json!({
            "district":format!("{district:?}"), "depth":depth,
            "unlabelledParentEdges":parent.iter().filter(|(_,label)|label.is_none()).count(),
            "kind":proposed_kind.as_str(), "parent":original, "split":split, "alternatives":proposals,
            "points":parent.iter().map(|(p,label)|serde_json::json!({"point":[p.x,p.z],"line":label})).collect::<Vec<_>>()
        })
    );
}

#[test]
fn preview_is_repeatable_and_reserves_new_street_land() {
    let parent = vec![
        (P::new(0.0, 0.0), Some(0)),
        (P::new(80.0, 0.0), Some(0)),
        (P::new(80.0, 50.0), Some(0)),
        (P::new(0.0, 50.0), Some(0)),
    ];
    let original = parent.clone();
    let kind = |_| RoadKind::Local;
    let before = evaluate(&[parent.clone()], District::Compact, &kind);
    assert_eq!(
        before,
        evaluate(&[parent.clone()], District::Compact, &kind)
    );
    let children: Vec<Poly> = [P::new(1.0, 0.0), P::new(-1.0, 0.0)]
        .into_iter()
        .map(|n| clip(&parent, P::new(40.0, 0.0), n, Some(PROPOSED_STREET)))
        .collect();
    let after = evaluate(&children, District::Compact, &kind);
    assert!(after.inset_area < before.inset_area);
    assert!((before.inset_area - after.inset_area - 7.2 * 42.8).abs() < 0.1);
    assert!(before.direct_lots_by_seed.iter().all(|&n| n > 0));
    assert_eq!(parent, original);
}

#[test]
fn concave_parent_keeps_both_frontage_wings() {
    let polygon = vec![
        (0.0, 0.0),
        (80.0, 0.0),
        (80.0, 30.0),
        (40.0, 30.0),
        (40.0, 70.0),
        (0.0, 70.0),
    ]
    .into_iter()
    .map(|(x, z)| (P::new(x, z), Some(0)))
    .collect();
    let preview = evaluate(&[polygon], District::Compact, &|_| RoadKind::Local);
    // Convex-only insetting wrongly intersects the two wings and loses most
    // of this block. Production decomposes before applying street setbacks.
    assert!(preview.inset_area > 2500.0 && preview.inset_area < 4000.0);
    for i in 0..3 {
        assert!(
            preview.direct_footprint_by_seed[i] + preview.rear_footprint_by_seed[i]
                <= preview.inset_area + 0.1
        );
    }
}

#[test]
#[cfg(feature = "street-placement-candidate")]
fn chosen_placement_preserves_hosts_and_generator_state() {
    let mut builder = Builder {
        nodes: vec![],
        index: BTreeMap::new(),
        roads: vec![],
        lines: vec![],
        line_lookup: BTreeMap::new(),
    };
    let points = [
        P::new(0.0, 0.0),
        P::new(80.0, 0.0),
        P::new(80.0, 100.0),
        P::new(0.0, 100.0),
    ];
    let ids: Vec<_> = points
        .iter()
        .map(|p| builder.node(*p, NodeKind::Junction))
        .collect();
    let lines: Vec<_> = (0..4)
        .map(|i| builder.line(ids[i], ids[(i + 1) % 4], RoadKind::Local))
        .collect();
    let parent: Poly = (0..4).map(|i| (points[i], Some(lines[i]))).collect();
    let a = clip(&parent, P::new(20.0, 0.0), P::new(1.0, 0.0), None);
    let b = clip(&parent, P::new(20.0, 0.0), P::new(-1.0, 0.0), None);
    let before = (
        builder.nodes.len(),
        builder
            .lines
            .iter()
            .map(|l| l.splits.clone())
            .collect::<Vec<_>>(),
    );
    let run = || {
        choose(
            &builder,
            &parent,
            &a,
            &b,
            [lines[0], lines[2]],
            District::Compact,
            RoadKind::Local,
            P::new(40.0, 50.0),
            P::new(1.0, 0.0),
            80.0,
            20.0,
            0.2,
        )
    };
    let chosen = run().expect("asymmetric baseline has a robust alternative");
    assert_eq!(Some(chosen.clone()), run());
    assert!((area(&chosen.0) + area(&chosen.1) - area(&parent)).abs() < 0.1);
    for (point, line) in chosen.2 {
        assert!(line == Some(lines[0]) || line == Some(lines[2]));
        assert!(builder.nearest_junction(line.unwrap(), point) > 14.0);
    }
    assert_eq!(
        before,
        (
            builder.nodes.len(),
            builder
                .lines
                .iter()
                .map(|l| l.splits.clone())
                .collect::<Vec<_>>()
        )
    );
    assert!(!chosen
        .0
        .iter()
        .chain(chosen.1.iter())
        .any(|(_, l)| *l == Some(PROPOSED_STREET)));
}
