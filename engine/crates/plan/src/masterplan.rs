//! Seeded master plan: transport skeleton, arterial lattice, ruled local
//! streets, curved bank boundaries and street-facing lot partitions.
//!
//! The plan is pure data derived from the seed and river geometry. It never
//! touches engine state or simulation RNG streams, so restore can regenerate it
//! exactly. Bank alignments retain sampled bends between shared junctions.
//! Local cuts stay inside block boundaries; concave blocks are decomposed into
//! convex pieces before parcel fitting, preserving actual street frontage.

use std::collections::BTreeMap;

#[cfg(test)]
mod frontage_prototype;
#[cfg(feature = "parent-passage-candidate")]
mod parent_passage;
#[cfg(any(test, feature = "street-placement-candidate"))]
mod street_frontage_preview;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct P {
    pub x: f32,
    pub z: f32,
}

impl P {
    fn new(x: f32, z: f32) -> Self {
        Self { x, z }
    }
    fn add(self, o: P) -> P {
        P::new(self.x + o.x, self.z + o.z)
    }
    fn sub(self, o: P) -> P {
        P::new(self.x - o.x, self.z - o.z)
    }
    fn scale(self, s: f32) -> P {
        P::new(self.x * s, self.z * s)
    }
    fn dot(self, o: P) -> f32 {
        self.x * o.x + self.z * o.z
    }
    fn cross(self, o: P) -> f32 {
        self.x * o.z - self.z * o.x
    }
    fn len(self) -> f32 {
        // sqrt is correctly rounded on every IEEE target; hypot is not, and
        // the plan must regenerate bit-identically in native and WASM.
        (self.x * self.x + self.z * self.z).sqrt()
    }
    fn lerp(self, o: P, t: f32) -> P {
        self.add(o.sub(self).scale(t))
    }
    fn unit(self) -> P {
        let l = self.len().max(1.0e-6);
        self.scale(1.0 / l)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Junction,
    Gateway,
    Station,
    RailGateway,
    RailTerminus,
    Elevated,
}

impl NodeKind {
    /// The world node kind this plan node becomes when realized.
    pub fn world(self) -> crate::kinds::NodeKind {
        match self {
            NodeKind::Junction => crate::kinds::NodeKind::Junction,
            NodeKind::Gateway => crate::kinds::NodeKind::Gateway,
            NodeKind::Station => crate::kinds::NodeKind::Station,
            NodeKind::RailGateway => crate::kinds::NodeKind::RailGateway,
            NodeKind::RailTerminus => crate::kinds::NodeKind::RailTerminus,
            NodeKind::Elevated => crate::kinds::NodeKind::Elevated,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            NodeKind::Junction => "junction",
            NodeKind::Gateway => "gateway",
            NodeKind::Station => "station",
            NodeKind::RailGateway => "rail_gateway",
            NodeKind::RailTerminus => "rail_terminus",
            NodeKind::Elevated => "elevated",
        }
    }
}

pub use crate::kinds::RoadKind;

impl RoadKind {
    /// Distance from the road centerline to the lot line: half the rendered
    /// carriageway plus its sidewalk.
    fn lot_setback(self) -> f32 {
        match self {
            // 16 m carriageway plus 2.5 m sidewalks.
            RoadKind::Arterial => 10.8,
            RoadKind::Local => 3.6,
            RoadKind::Bridge => 6.0,
            RoadKind::Lane => 2.3,
            RoadKind::Rail | RoadKind::Expressway => 9.0,
        }
    }
    pub fn is_street(self) -> bool {
        matches!(
            self,
            RoadKind::Arterial | RoadKind::Local | RoadKind::Lane | RoadKind::Bridge
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum District {
    StationCore,
    StationFringe,
    Compact,
    Strips,
    Waterfront,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanNode {
    pub p: P,
    pub kind: NodeKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanRoad {
    pub a: usize,
    pub b: usize,
    pub kind: RoadKind,
    pub deck_height: f32,
    pub curve: Option<[P; 2]>,
}

mod flag_passage;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanLot {
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub depth: f32,
    /// Three.js Y rotation; local +Z faces the frontage road.
    pub rotation: f32,
    /// Unit vector the lot faces (toward its frontage road). Plan decisions
    /// use this instead of trigonometry so they are target-independent.
    pub facing: P,
    pub road: usize,
    pub district: District,
    /// True when the frontage road is an arterial.
    pub arterial_frontage: bool,
    /// Rear flag lot reached by a passage beside a front lot.
    pub flag: bool,
    /// Ordered rear threshold / bend / street entrance, reserved before fitting.
    pub passage: Option<[P; 3]>,
    pub block: u32,
}

impl PlanLot {
    /// A reserved passage can serve a building facing away from its street.
    /// Keep this isolated until parent-level reservation/admission is validated.
    pub fn passage_rotation(&self) -> Option<f32> {
        (cfg!(feature = "parent-passage-candidate") && self.passage.is_some())
            .then_some(self.rotation)
    }
}

/// Stage-area witnesses for diagnosing unassigned land without changing generation.
#[cfg(test)]
#[derive(Clone, Debug, serde::Serialize)]
struct BlockLandBudget {
    block: u32,
    inset_area: f32,
    after_roji_area: f32,
    after_elevated_area: f32,
    pending_area: f32,
}

/// Includes collapsed blocks, which never reach the parcel budget.
#[cfg(test)]
#[derive(Clone, Debug, serde::Serialize)]
struct StreetLandBudget {
    district: String,
    points: Vec<[f32; 2]>,
    edge_kinds: Vec<Option<String>>,
    raw_area: f32,
    after_setback_area: f32,
    after_curve_trim_area: f32,
}

/// Subdivision provenance is diagnostic only; absent from production state.
#[cfg(test)]
#[derive(Clone, Debug, serde::Serialize)]
struct StreetSubdivision {
    coarse_cell: [usize; 2],
    input_junctions: Vec<Vec<[f32; 2]>>,
    depth: u32,
    district: String,
    parent: Vec<[f32; 2]>,
    line: usize,
    host_lines: [usize; 2],
    endpoints: [[f32; 2]; 2],
    kind: String,
    roads: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct MasterPlan {
    pub rail_alignment: Option<crate::rail_alignment::Alignment>,
    /// Test/export-only witnesses for final admission losses; never enters WASM state.
    #[cfg(test)]
    rejected_lot_ground: Vec<(&'static str, Vec<P>)>,
    #[cfg(test)]
    block_land_budget: Vec<BlockLandBudget>,
    #[cfg(test)]
    street_land_budget: Vec<StreetLandBudget>,
    #[cfg(test)]
    street_subdivisions: Vec<StreetSubdivision>,
    #[cfg(test)]
    partition_rejected_ground: Vec<(&'static str, Vec<P>)>,
    pub nodes: Vec<PlanNode>,
    pub roads: Vec<PlanRoad>,
    pub lots: Vec<PlanLot>,
    /// Clipped partition polygon per accepted lot, retained for progressive ground.
    pub lot_ground: Vec<Vec<P>>,
    /// Buildable (inset) block outline per block id.
    pub blocks: Vec<Vec<P>>,
    /// Shared street-block identity before convex decomposition. A seam between
    /// equal parents is a clipping boundary, not automatically an access barrier.
    pub block_parents: Vec<u32>,
    pub gateway: usize,
    pub station: usize,
    pub rail_gateway: usize,
    pub rail_terminus: usize,
    /// Plan road indices opened at founding, ordered from the gateway.
    pub founding_roads: Vec<usize>,
    /// East-bank junction where the founding market street meets the river.
    pub founding_market: usize,
    /// Service rail (station → rail gateway) and the western continuation.
    pub rail_service: usize,
    pub rail_extension: usize,
    pub expressway: Vec<usize>,
}

pub struct Geography {
    pub seed: u32,
    /// River centerline control points, ascending in z.
    pub river: Vec<P>,
    pub river_clearance: f32,
    /// Axis-aligned protected rectangle (min_x, min_z, max_x, max_z).
    pub protected: (f32, f32, f32, f32),
}

pub const PLAN_EXTENT: f32 = 382.0;
pub const EDGE: f32 = 392.0;
pub const MAX_PLAN_LOTS: usize = 4_000;
const BANK_OFFSET: f32 = 47.0;
const RAIL_BAND_HALF: f32 = 11.0;

struct Rng(u64);

impl Rng {
    fn new(seed: u32) -> Self {
        Rng((seed as u64) ^ 0x6d61_7374_6572_706c)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
}

/// Parcel and alley choices belong to a geometric block, not its transient ID
/// or the amount of work consumed while subdividing earlier blocks. Quantize at
/// the planner's millimetre precision and sort to ignore winding/start vertex.
fn block_rng(seed: u32, block: &Poly) -> Rng {
    let mut points: Vec<_> = block
        .iter()
        .map(|(p, _)| ((p.x * 1000.0).round() as i32, (p.z * 1000.0).round() as i32))
        .collect();
    points.sort_unstable();
    let mut hash = 0x7061_7263_656c_0001u64 ^ seed as u64;
    for (x, z) in points {
        for word in [x as u32, z as u32] {
            hash ^= word as u64;
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    Rng(hash)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Ring,
    Arterial,
    /// Lattice line built as a local street: Tokyo arterials are far apart,
    /// so most superblock edges are ordinary streets.
    Collector,
    RailFrontage,
}

impl LineKind {
    fn road(self) -> RoadKind {
        match self {
            LineKind::Arterial => RoadKind::Arterial,
            LineKind::Ring | LineKind::Collector | LineKind::RailFrontage => RoadKind::Local,
        }
    }
}

/// A street alignment, straight or sampled. Joining streets split it into
/// road segments that retain shared junction nodes and the sampled bends.
struct Line {
    a: usize,
    b: usize,
    kind: RoadKind,
    splits: Vec<(f32, usize)>,
    /// Resolved road indices in order from `a` to `b`, with their end
    /// parameters.
    roads: Vec<(f32, f32, usize)>,
    path: Vec<P>,
}

/// Convex polygon whose edge `i` (vertex `i` → `i + 1`) carries a label: the
/// street line it lies on, or `None` for an internal lot boundary.
type Poly = Vec<(P, Option<usize>)>;

struct PendingLot {
    lot: PlanLot,
    ground: Vec<P>,
    front_mid: P,
    line: usize,
}

struct Builder {
    nodes: Vec<PlanNode>,
    index: BTreeMap<(i64, i64), usize>,
    roads: Vec<PlanRoad>,
    lines: Vec<Line>,
    line_lookup: BTreeMap<(usize, usize), usize>,
}

impl Builder {
    fn node(&mut self, p: P, kind: NodeKind) -> usize {
        let key = ((p.x * 100.0).round() as i64, (p.z * 100.0).round() as i64);
        if let Some(&id) = self.index.get(&key) {
            return id;
        }
        let id = self.nodes.len();
        self.nodes.push(PlanNode { p, kind });
        self.index.insert(key, id);
        id
    }
    fn road(&mut self, a: usize, b: usize, kind: RoadKind) -> usize {
        let deck_height = if kind == RoadKind::Expressway {
            11.0
        } else {
            0.0
        };
        self.roads.push(PlanRoad {
            a,
            b,
            kind,
            deck_height,
            curve: None,
        });
        self.roads.len() - 1
    }
    fn line(&mut self, a: usize, b: usize, kind: RoadKind) -> usize {
        let key = (a.min(b), a.max(b));
        if let Some(&id) = self.line_lookup.get(&key) {
            return id;
        }
        self.lines.push(Line {
            a,
            b,
            kind,
            splits: Vec::new(),
            roads: Vec::new(),
            path: Vec::new(),
        });
        let id = self.lines.len() - 1;
        self.line_lookup.insert(key, id);
        id
    }
    fn line_param(&self, line: usize, p: P) -> f32 {
        if !self.lines[line].path.is_empty() {
            return path_param(&self.lines[line].path, p);
        }
        let (a, b) = (
            self.nodes[self.lines[line].a].p,
            self.nodes[self.lines[line].b].p,
        );
        let d = b.sub(a);
        p.sub(a).dot(d) / d.dot(d)
    }
    fn split(&mut self, line: usize, p: P) -> usize {
        let t = self.line_param(line, p).clamp(0.0, 1.0);
        if t <= 1.0e-4 {
            return self.lines[line].a;
        }
        if t >= 1.0 - 1.0e-4 {
            return self.lines[line].b;
        }
        let id = self.node(p, NodeKind::Junction);
        if !self.lines[line].splits.iter().any(|(_, n)| *n == id) {
            self.lines[line].splits.push((t, id));
        }
        id
    }
    /// Distance from `p` to the nearest existing junction on `line`.
    fn nearest_junction(&self, line: usize, p: P) -> f32 {
        let l = &self.lines[line];
        std::iter::once(l.a)
            .chain(std::iter::once(l.b))
            .chain(l.splits.iter().map(|s| s.1))
            .map(|n| self.nodes[n].p.sub(p).len())
            .fold(f32::INFINITY, f32::min)
    }
    fn resolve_lines(&mut self, protected: (f32, f32, f32, f32)) {
        for e in 0..self.lines.len() {
            let mut points = vec![(0.0_f32, self.lines[e].a)];
            let mut splits = self.lines[e].splits.clone();
            for p in self.lines[e].path.clone().into_iter().skip(1).rev().skip(1) {
                let id = self.node(p, NodeKind::Junction);
                splits.push((self.line_param(e, p), id));
            }
            splits.sort_by(|x, y| x.0.total_cmp(&y.0));
            points.extend(splits);
            points.push((1.0, self.lines[e].b));
            points.dedup_by(|x, y| x.1 == y.1);
            let kind = self.lines[e].kind;
            let mut resolved = Vec::new();
            for pair in points.windows(2) {
                let (pa, pb) = (self.nodes[pair[0].1].p, self.nodes[pair[1].1].p);
                let minimum_length = 1.0;
                let minimum_length = if self.lines[e].path.is_empty() {
                    minimum_length
                } else {
                    0.001
                };
                if pa.sub(pb).len() < minimum_length || segment_hits_rect(pa, pb, protected, 0.0) {
                    continue;
                }
                let road = self.road(pair[0].1, pair[1].1, kind);
                resolved.push((pair[0].0, pair[1].0, road));
            }
            self.lines[e].roads = resolved;
        }
    }
}

fn segment_hits_rect(a: P, b: P, r: (f32, f32, f32, f32), margin: f32) -> bool {
    (0..=24).any(|i| {
        let p = a.lerp(b, i as f32 / 24.0);
        p.x >= r.0 - margin && p.x <= r.2 + margin && p.z >= r.1 - margin && p.z <= r.3 + margin
    })
}

fn point_segment_distance(p: P, a: P, b: P) -> f32 {
    let d = b.sub(a);
    let t = (p.sub(a).dot(d) / d.dot(d).max(1.0e-9)).clamp(0.0, 1.0);
    p.sub(a.lerp(b, t)).len()
}

fn path_param(path: &[P], p: P) -> f32 {
    let total: f32 = path.windows(2).map(|w| w[1].sub(w[0]).len()).sum();
    let mut passed = 0.0;
    let mut best = (f32::INFINITY, 0.0);
    for w in path.windows(2) {
        let d = w[1].sub(w[0]);
        let length = d.len();
        let t = p.sub(w[0]).dot(d) / d.dot(d);
        let distance = p.sub(w[0].lerp(w[1], t.clamp(0.0, 1.0))).len();
        if distance < best.0 {
            best = (distance, (passed + t.clamp(0.0, 1.0) * length) / total);
        }
        passed += length;
    }
    best.1
}

fn path_position(path: &[P], t: f32) -> P {
    let total: f32 = path.windows(2).map(|w| w[1].sub(w[0]).len()).sum();
    let mut remaining = t.clamp(0.0, 1.0) * total;
    for w in path.windows(2) {
        let length = w[1].sub(w[0]).len();
        if remaining <= length {
            return w[0].lerp(w[1], remaining / length);
        }
        remaining -= length;
    }
    *path.last().expect("sampled street path")
}

/// Insetting a decomposed corner can extend a parallel frontage edge beyond
/// its actual road segment. Keep that land, but label only the finite portion
/// which has a road in front of it.
fn trim_curve_frontage(poly: Poly, builder: &Builder) -> Poly {
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let (a, label) = poly[i];
        let end = poly[(i + 1) % poly.len()].0;
        let edge = end.sub(a);
        let path = label
            .map(|l| builder.lines[l].path.as_slice())
            .unwrap_or(&[]);
        if path.is_empty() {
            out.push((a, label));
            continue;
        }
        let matching = path
            .windows(2)
            .filter(|w| edge.unit().dot(w[1].sub(w[0]).unit()).abs() > 0.99999)
            .min_by(|x, y| {
                point_segment_distance(a.lerp(end, 0.5), x[0], x[1])
                    .total_cmp(&point_segment_distance(a.lerp(end, 0.5), y[0], y[1]))
            });
        let Some(w) = matching else {
            out.push((a, None));
            continue;
        };
        let u = w[1].sub(w[0]).unit();
        let t0 = w[0].sub(a).dot(u) / edge.dot(u);
        let t1 = w[1].sub(a).dot(u) / edge.dot(u);
        let lo = t0.min(t1).max(0.0);
        let hi = t0.max(t1).min(1.0);
        if hi <= lo {
            out.push((a, None));
            continue;
        }
        if lo > 0.0001 {
            out.push((a, None));
            out.push((a.lerp(end, lo), label));
        } else {
            out.push((a, label));
        }
        if hi < 0.9999 {
            out.push((a.lerp(end, hi), None));
        }
    }
    out
}

/// Split only reflex corners; internal diagonals carry no street frontage.
/// Greedily merge adjacent triangles back into convex pieces before lots are
/// inset, avoiding a thin parcel strip at every curve sample.
fn convex_pieces(poly: Poly) -> Vec<Poly> {
    fn convex(p: &Poly) -> bool {
        (0..p.len()).all(|i| {
            let a = p[i].0;
            let b = p[(i + 1) % p.len()].0;
            let c = p[(i + 2) % p.len()].0;
            b.sub(a).cross(c.sub(b)) >= -0.001
        })
    }
    if convex(&poly) {
        return vec![poly];
    }
    let mut remaining = poly;
    let mut pieces = Vec::new();
    while remaining.len() > 3 {
        let n = remaining.len();
        let ear = (0..n)
            .find(|&i| {
                let a = remaining[(i + n - 1) % n].0;
                let b = remaining[i].0;
                let c = remaining[(i + 1) % n].0;
                if b.sub(a).cross(c.sub(b)) <= 0.001 {
                    return false;
                }
                !(0..n)
                    .filter(|&j| j != i && j != (i + n - 1) % n && j != (i + 1) % n)
                    .any(|j| {
                        let p = remaining[j].0;
                        b.sub(a).cross(p.sub(a)) >= -0.001
                            && c.sub(b).cross(p.sub(b)) >= -0.001
                            && a.sub(c).cross(p.sub(c)) >= -0.001
                    })
            })
            .expect("simple bank block must have an ear");
        let prev = (ear + n - 1) % n;
        let next = (ear + 1) % n;
        pieces.push(vec![
            remaining[prev],
            remaining[ear],
            (remaining[next].0, None),
        ]);
        remaining[prev].1 = None;
        remaining.remove(ear);
    }
    pieces.push(remaining);
    'merge: loop {
        for a in 0..pieces.len() {
            for b in a + 1..pieces.len() {
                for i in 0..pieces[a].len() {
                    for j in 0..pieces[b].len() {
                        let na = pieces[a].len();
                        let nb = pieces[b].len();
                        if pieces[a][i].1.is_some()
                            || pieces[b][j].1.is_some()
                            || pieces[a][i].0 != pieces[b][(j + 1) % nb].0
                            || pieces[a][(i + 1) % na].0 != pieces[b][j].0
                        {
                            continue;
                        }
                        let joined: Poly = (1..na)
                            .map(|k| pieces[a][(i + k) % na])
                            .chain((1..nb).map(|k| pieces[b][(j + k) % nb]))
                            .collect();
                        if convex(&joined) {
                            pieces[a] = joined;
                            pieces.remove(b);
                            continue 'merge;
                        }
                    }
                }
            }
        }
        break;
    }
    pieces
}

/// Distance from a segment to a convex quad, using only IEEE-exact operations
/// so plan decisions match on every target.
fn quad_segment_distance(q: &[P; 4], a: P, b: P) -> f32 {
    let inside = |p: P| {
        let signs: Vec<f32> = (0..4)
            .map(|i| q[(i + 1) % 4].sub(q[i]).cross(p.sub(q[i])))
            .collect();
        signs.iter().all(|s| *s >= 0.0) || signs.iter().all(|s| *s <= 0.0)
    };
    if inside(a) || inside(b) {
        return 0.0;
    }
    let crosses = |p1: P, p2: P, p3: P, p4: P| {
        let d1 = p2.sub(p1).cross(p3.sub(p1));
        let d2 = p2.sub(p1).cross(p4.sub(p1));
        let d3 = p4.sub(p3).cross(p1.sub(p3));
        let d4 = p4.sub(p3).cross(p2.sub(p3));
        d1 * d2 < 0.0 && d3 * d4 < 0.0
    };
    let mut best = f32::INFINITY;
    for i in 0..4 {
        let (c, d) = (q[i], q[(i + 1) % 4]);
        if crosses(a, b, c, d) {
            return 0.0;
        }
        best = best
            .min(point_segment_distance(c, a, b))
            .min(point_segment_distance(a, c, d))
            .min(point_segment_distance(b, c, d));
    }
    best
}

fn lot_clearance_rejection(
    lot: &PlanLot,
    geo: &Geography,
    rail_alignment: Option<&crate::rail_alignment::Alignment>,
    rail_z: f32,
    nodes: &[PlanNode],
    roads: &[PlanRoad],
    elevated_clearance: &[(P, P, f32)],
    access_road: Option<usize>,
) -> Option<&'static str> {
    let river = &geo.river;
    let corners = rect_corners(&lot);
    let pr = geo.protected;
    if corners.iter().any(|p| {
        river_distance(river, *p) < geo.river_clearance
            || p.x.abs() > EDGE - 4.0
            || p.z.abs() > EDGE - 4.0
            || (rail_alignment.is_none() && (p.z - rail_z).abs() < RAIL_BAND_HALF - 0.5)
            || (p.x >= pr.0 - 2.0 && p.x <= pr.2 + 2.0 && p.z >= pr.1 - 2.0 && p.z <= pr.3 + 2.0)
    }) {
        return Some("geography");
    }
    if rail_alignment.as_ref().is_some_and(|a| {
        a.samples().windows(2).any(|pair| {
            quad_segment_distance(&corners, pair[0].1, pair[1].1) < RAIL_BAND_HALF - 0.05
        })
    }) {
        return Some("rail_clearance");
    }
    // Every street keeps its setback, including streets that continue
    // past a block corner at an angle (bent arterials beyond a collector).
    let corners_exact = rect_corners(&lot);
    let reach = lot.width.max(lot.depth) + 12.0;
    let intrudes = roads.iter().enumerate().any(|(index, road)| {
        if access_road == Some(index) {
            return false;
        }
        if !road.kind.is_street() {
            return false;
        }
        let (a, b2) = (nodes[road.a].p, nodes[road.b].p);
        if a.x.min(b2.x) > lot.x + reach
            || a.x.max(b2.x) < lot.x - reach
            || a.z.min(b2.z) > lot.z + reach
            || a.z.max(b2.z) < lot.z - reach
        {
            return false;
        }
        // Only float-scale slack is permitted. A half-metre allowance
        // admitted lots inside a lane's runtime clearance envelope.
        quad_segment_distance(&corners_exact, a, b2) < road.kind.lot_setback() - 0.05
    });
    if intrudes {
        return Some("ground_street_clearance");
    }
    if elevated_clearance.iter().any(|&(a, b, clearance)| {
        if a.x.min(b.x) > lot.x + reach + clearance
            || a.x.max(b.x) < lot.x - reach - clearance
            || a.z.min(b.z) > lot.z + reach + clearance
            || a.z.max(b.z) < lot.z - reach - clearance
        {
            return false;
        }
        quad_segment_distance(&corners_exact, a, b) < clearance
    }) {
        return Some("elevated_clearance");
    }
    None
}

fn rect_corners(lot: &PlanLot) -> [P; 4] {
    // Local +Z is the facing (sin r, cos r); local +X is (cos r, -sin r).
    let f = lot.facing;
    let ux = P::new(f.z, -f.x).scale(lot.width * 0.5);
    let uz = f.scale(lot.depth * 0.5);
    let o = P::new(lot.x, lot.z);
    [
        o.sub(ux).sub(uz),
        o.add(ux).sub(uz),
        o.add(ux).add(uz),
        o.sub(ux).add(uz),
    ]
}

pub fn river_x(river: &[P], z: f32) -> f32 {
    for w in river.windows(2) {
        if z >= w[0].z && z <= w[1].z {
            let t = ((z - w[0].z) / (w[1].z - w[0].z)).clamp(0.0, 1.0);
            return w[0].x + (w[1].x - w[0].x) * t;
        }
    }
    if z < river[0].z {
        river[0].x
    } else {
        river.last().unwrap().x
    }
}

fn river_distance(river: &[P], p: P) -> f32 {
    river
        .windows(2)
        .map(|w| {
            let d = w[1].sub(w[0]);
            let t = (p.sub(w[0]).dot(d) / d.dot(d)).clamp(0.0, 1.0);
            p.sub(w[0].lerp(w[1], t)).len()
        })
        .fold(f32::INFINITY, f32::min)
}

fn area(poly: &Poly) -> f32 {
    let n = poly.len();
    (0..n)
        .map(|i| poly[i].0.cross(poly[(i + 1) % n].0))
        .sum::<f32>()
        * 0.5
}

fn centroid(poly: &Poly) -> P {
    let n = poly.len() as f32;
    poly.iter()
        .fold(P::new(0.0, 0.0), |acc, v| acc.add(v.0))
        .scale(1.0 / n)
}

/// Make the polygon counter-clockwise (positive area) so inward normals are
/// consistent.
fn ccw(mut poly: Poly) -> Poly {
    if area(&poly) < 0.0 {
        // Reverse vertex order; edge labels shift with their edges.
        let n = poly.len();
        let labels: Vec<Option<usize>> = (0..n).map(|i| poly[i].1).collect();
        let points: Vec<P> = poly.iter().map(|v| v.0).collect();
        poly = (0..n)
            .map(|i| {
                let p = points[n - 1 - i];
                // Edge from points[n-1-i] to points[n-2-i] was edge n-2-i.
                let label = labels[(2 * n - 2 - i) % n];
                (p, label)
            })
            .collect();
    }
    poly
}

/// Keep the part of a convex polygon where `dot(p - c, n) >= 0`. The new edge
/// along the cut line gets `cut_label`.
fn clip(poly: &Poly, c: P, n: P, cut_label: Option<usize>) -> Poly {
    let len = poly.len();
    let mut out: Poly = Vec::with_capacity(len + 1);
    for i in 0..len {
        let (p, label) = poly[i];
        let q = poly[(i + 1) % len].0;
        let sp = p.sub(c).dot(n);
        let sq = q.sub(c).dot(n);
        if sp >= 0.0 {
            out.push((p, label));
        }
        if (sp >= 0.0) != (sq >= 0.0) {
            let t = sp / (sp - sq);
            let x = p.lerp(q, t);
            // Entering the kept side continues the original edge; leaving it
            // starts the edge along the cut.
            out.push((x, if sp >= 0.0 { cut_label } else { label }));
        }
    }
    // Drop near-duplicate vertices.
    let mut cleaned: Poly = Vec::with_capacity(out.len());
    for v in out {
        if cleaned
            .last()
            .is_some_and(|last: &(P, Option<usize>)| last.0.sub(v.0).len() < 0.05)
        {
            let last = cleaned.len() - 1;
            cleaned[last].1 = v.1;
            continue;
        }
        cleaned.push(v);
    }
    if cleaned.len() > 2 && cleaned[0].0.sub(cleaned[cleaned.len() - 1].0).len() < 0.05 {
        cleaned.pop();
    }
    cleaned
}

/// Oriented bounding box: (long-axis unit, perpendicular unit, center, long
/// extent, short extent).
fn obb(poly: &Poly) -> (P, P, P, f32, f32) {
    let mut best = (f32::INFINITY, P::new(1.0, 0.0), P::new(0.0, 0.0), 0.0, 0.0);
    let n = poly.len();
    for i in 0..n {
        let d = poly[(i + 1) % n].0.sub(poly[i].0);
        if d.len() < 0.5 {
            continue;
        }
        let u = d.unit();
        let v = P::new(-u.z, u.x);
        let (mut u0, mut u1, mut v0, mut v1) = (
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
        );
        for (p, _) in poly {
            let (pu, pv) = (p.dot(u), p.dot(v));
            u0 = u0.min(pu);
            u1 = u1.max(pu);
            v0 = v0.min(pv);
            v1 = v1.max(pv);
        }
        let a = (u1 - u0) * (v1 - v0);
        if a < best.0 {
            let center = u.scale((u0 + u1) * 0.5).add(v.scale((v0 + v1) * 0.5));
            best = (a, u, center, u1 - u0, v1 - v0);
        }
    }
    let (_, u, center, lu, lv) = best;
    let v = P::new(-u.z, u.x);
    if lu >= lv {
        (u, v, center, lu, lv)
    } else {
        (v, u.scale(-1.0), center, lv, lu)
    }
}

/// Shrink a convex polygon by a per-edge setback, keeping street labels on the
/// offset edges.
fn inset(poly: &Poly, setback: impl Fn(Option<usize>) -> f32) -> Poly {
    let mut out = poly.clone();
    let n = poly.len();
    for i in 0..n {
        let (a, label) = poly[i];
        let b = poly[(i + 1) % n].0;
        let d = b.sub(a).unit();
        let inward = P::new(-d.z, d.x);
        let c = a.add(inward.scale(setback(label)));
        out = clip(&out, c, inward, label);
        if out.len() < 3 {
            return Vec::new();
        }
    }
    out
}

/// Reserve continuous sides of each route within a block. The intersection of
/// offset tangent half-planes keeps each side convex, avoiding a sliver per sample.
/// Cut edges are never street frontage. Final exact clearance remains authoritative.
fn reserve_elevated_corridor(poly: Poly, segments: &[(P, P, f32)]) -> Vec<Poly> {
    let mut routes: Vec<Vec<(P, P, f32)>> = Vec::new();
    for &(a, b, r) in segments {
        if routes
            .last()
            .and_then(|route| route.last())
            .is_none_or(|last| last.1.sub(a).len() > 0.01)
        {
            routes.push(Vec::new());
        }
        routes.last_mut().unwrap().push((a, b, r));
    }
    let mut pieces = vec![poly];
    for route in routes {
        let mut next = Vec::new();
        for piece in pieces {
            let min_x = piece.iter().map(|p| p.0.x).fold(f32::INFINITY, f32::min);
            let max_x = piece
                .iter()
                .map(|p| p.0.x)
                .fold(f32::NEG_INFINITY, f32::max);
            let min_z = piece.iter().map(|p| p.0.z).fold(f32::INFINITY, f32::min);
            let max_z = piece
                .iter()
                .map(|p| p.0.z)
                .fold(f32::NEG_INFINITY, f32::max);
            let nearby: Vec<_> = route
                .iter()
                .filter(|&&(a, b, r)| {
                    a.x.min(b.x) - r <= max_x
                        && a.x.max(b.x) + r >= min_x
                        && a.z.min(b.z) - r <= max_z
                        && a.z.max(b.z) + r >= min_z
                })
                .collect();
            if nearby.is_empty() {
                next.push(piece);
                continue;
            }
            for side in [-1.0, 1.0] {
                let mut outside = piece.clone();
                for &&(a, b, r) in &nearby {
                    let u = b.sub(a).unit();
                    let n = P::new(-u.z, u.x).scale(side);
                    outside = clip(&outside, a.add(n.scale(r)), n, None);
                    if outside.len() < 3 {
                        break;
                    }
                }
                if outside.len() >= 3 && area(&outside) > 0.01 {
                    next.push(outside);
                }
            }
        }
        pieces = next;
    }
    pieces
}

fn frontage_range(district: District, arterial: bool) -> (f32, f32) {
    match (district, arterial) {
        (District::StationCore, _) => (9.0, 16.0),
        (_, true) => (8.0, 13.0),
        (District::StationFringe, false) => (6.5, 11.0),
        (District::Waterfront, false) => (9.0, 15.0),
        (District::Compact | District::Strips, false) => (5.6, 8.6),
    }
}

/// Largest rectangle inside a convex lot whose front side lies on the street
/// edge `edge`. Returns (center, width, depth, outward normal).
fn inscribed_rect(poly: &Poly, edge: usize, max_depth: f32) -> Option<(P, f32, f32, P)> {
    let n = poly.len();
    let a = poly[edge].0;
    let u = poly[(edge + 1) % n].0.sub(a).unit();
    let mut v = P::new(-u.z, u.x);
    if centroid(poly).sub(a).dot(v) < 0.0 {
        v = v.scale(-1.0);
    }
    inscribed_rect_frame(poly, a, u, v, max_depth)
}

/// Prefer the longest real frontage, but a narrow corner lot may only fit a
/// building when it faces its other street. Never turn a subdivision cut into access.
fn fit_street_rectangle(
    poly: &Poly,
    max_depth: f32,
    allow_fallback: bool,
) -> Option<(usize, usize, P, f32, f32, P)> {
    let mut edges: Vec<_> = (0..poly.len())
        .filter_map(|i| {
            poly[i]
                .1
                .map(|line| (i, line, poly[(i + 1) % poly.len()].0.sub(poly[i].0).len()))
        })
        .filter(|(_, _, length)| *length >= 4.0)
        .collect();
    // max_by previously preferred the last edge on a length tie.
    edges.sort_by(|a, b| b.2.total_cmp(&a.2).then_with(|| b.0.cmp(&a.0)));
    if !allow_fallback {
        edges.truncate(1);
    }
    edges.into_iter().find_map(|(edge, line, _)| {
        inscribed_rect(poly, edge, max_depth).map(|(p, w, d, facing)| (edge, line, p, w, d, facing))
    })
}

/// Largest rectangle inside a convex polygon with its front side on the line
/// through `a` along `u`, extending toward `v`.
fn inscribed_rect_frame(poly: &Poly, a: P, u: P, v: P, max_depth: f32) -> Option<(P, f32, f32, P)> {
    let n = poly.len();
    // Slice of the polygon at depth `d`: interval in u.
    let slice = |d: f32| -> Option<(f32, f32)> {
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for i in 0..n {
            let p = poly[i].0.sub(a);
            let q = poly[(i + 1) % n].0.sub(a);
            let (pu, pv) = (p.dot(u), p.dot(v));
            let (qu, qv) = (q.dot(u), q.dot(v));
            if (pv - d) * (qv - d) <= 0.0 && (pv - qv).abs() > 1.0e-6 {
                let t = (d - pv) / (qv - pv);
                let x = pu + (qu - pu) * t;
                lo = lo.min(x);
                hi = hi.max(x);
            } else if (pv - d).abs() < 1.0e-6 {
                lo = lo.min(pu);
                hi = hi.max(pu);
            }
        }
        (hi > lo).then_some((lo, hi))
    };
    let deepest = poly
        .iter()
        .map(|p| p.0.sub(a).dot(v))
        .fold(0.0, f32::max)
        .min(max_depth);
    let front = slice(0.02)?;
    let mut best: Option<(f32, f32, f32, f32)> = None;
    for step in 1..=24 {
        let d = deepest * step as f32 / 24.0;
        let Some(back) = slice((d - 0.02).max(0.02)) else {
            continue;
        };
        let lo = front.0.max(back.0);
        let hi = front.1.min(back.1);
        let w = hi - lo;
        if w < 5.0 || d < 7.0 {
            continue;
        }
        if best.is_none_or(|b| w * d > b.0) {
            best = Some((w * d, lo, hi, d));
        }
    }
    let (_, lo, hi, d) = best?;
    let center = a.add(u.scale((lo + hi) * 0.5)).add(v.scale(d * 0.5));
    Some((center, hi - lo, d, v.scale(-1.0)))
}

fn extent(poly: &Poly, axis: P) -> (f32, f32) {
    poly.iter()
        .map(|p| p.0.dot(axis))
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), x| {
            (lo.min(x), hi.max(x))
        })
}

/// Direction and length of the longest street-facing edge.
fn longest_street_edge(poly: &Poly) -> Option<(P, f32)> {
    let n = poly.len();
    (0..n)
        .filter(|i| poly[*i].1.is_some())
        .map(|i| {
            let d = poly[(i + 1) % n].0.sub(poly[i].0);
            (d.unit(), d.len())
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, len)| *len >= 6.0)
}

/// Subdivide a buildable block into lots (Vanegas-style OBB splitting): first
/// back-to-back rows along the long axis, then frontage cuts across each row.
/// Lots without a street edge are dropped; they would need a lane.
fn partition_block(
    block: &Poly,
    district: District,
    block_id: u32,
    is_arterial: &dyn Fn(usize) -> bool,
    rng: &mut Rng,
    out: &mut Vec<PendingLot>,
    alternate_frontage: bool,
    #[cfg(test)] losses: &mut Vec<(&'static str, Vec<P>)>,
) {
    #[cfg(test)]
    if std::env::var("PLAN_TRACE_PARTITION_BLOCK")
        .ok()
        .is_some_and(|s| {
            s.split(',')
                .any(|value| value.parse::<u32>().ok() == Some(block_id))
        })
    {
        eprintln!(
            "PARTITION_INPUT {}",
            serde_json::json!({
                "block": block_id, "district": format!("{district:?}"),
                "points": block.iter().map(|(p, label)| serde_json::json!({
                    "point": [p.x,p.z], "line": label,
                    "arterial": label.is_some_and(is_arterial)
                })).collect::<Vec<_>>()
            })
        );
    }
    macro_rules! lost {
        ($reason:expr, $poly:expr) => {
            #[cfg(test)]
            losses.push(($reason, $poly.iter().map(|v| v.0).collect()));
            #[cfg(test)]
            if std::env::var("PLAN_TRACE_LOSS_REASON").as_deref() == Ok($reason) {
                eprintln!("PARTITION_LOSS {}", serde_json::json!({
                    "block": block_id, "district": format!("{district:?}"),
                    "reason": $reason,
                    "points": $poly.iter().map(|(p, label)| serde_json::json!({
                        "point": [p.x,p.z], "line": label,
                        "arterial": label.is_some_and(is_arterial)
                    })).collect::<Vec<_>>()
                }));
            }
        };
    }
    if area(block) < 60.0 {
        lost!("small_block", block);
        return;
    }
    // Rows follow the block's longest street edge, not its bounding box, so
    // lots line up with the street they face.
    let Some((u, _)) = longest_street_edge(block) else {
        lost!("block_without_frontage", block);
        return;
    };
    let v = P::new(-u.z, u.x);
    let (v0, v1) = extent(block, v);
    let lv = v1 - v0;
    let rows: Vec<Poly> = if lv >= 22.0 {
        let c = v.scale(v0 + lv * (0.5 + rng.range(-0.06, 0.06)));
        vec![clip(block, c, v, None), clip(block, c, v.scale(-1.0), None)]
    } else {
        vec![block.clone()]
    };
    let max_depth = match district {
        District::StationCore | District::Waterfront => 32.0,
        District::StationFringe => 30.0,
        District::Compact | District::Strips => 28.0,
    };
    let front_depth = match district {
        District::StationCore | District::Waterfront => 26.0,
        District::StationFringe => 18.0,
        District::Compact => 15.0,
        District::Strips => 14.0,
    };
    let cut_strip = |strip: Poly, ru: P, min_w: f32, max_w: f32, rng: &mut Rng| -> Vec<Poly> {
        let (u0, u1) = extent(&strip, ru);
        let span = u1 - u0;
        let mut cuts = Vec::new();
        let mut used = 0.0;
        loop {
            let w = rng.range(min_w, max_w);
            if span - (used + w) < min_w {
                break;
            }
            used += w;
            cuts.push(u0 + used);
        }
        let mut remaining = strip;
        let mut pieces = Vec::new();
        for x in cuts {
            let c = ru.scale(x);
            let piece = clip(&remaining, c, ru.scale(-1.0), None);
            remaining = clip(&remaining, c, ru, None);
            if piece.len() >= 3 {
                pieces.push(piece);
            }
            if remaining.len() < 3 {
                break;
            }
        }
        if remaining.len() >= 3 {
            pieces.push(remaining);
        }
        pieces
    };
    for row in rows {
        if row.len() < 3 || area(&row) < 40.0 {
            lost!("small_row", row);
            continue;
        }
        let n_row = row.len();
        let Some((row_edge, row_line)) = (0..n_row)
            .filter_map(|i| {
                row[i]
                    .1
                    .map(|line| (i, line, row[(i + 1) % n_row].0.sub(row[i].0).len()))
            })
            .max_by(|x, y| x.2.total_cmp(&y.2))
            .filter(|x| x.2 >= 6.0)
            .map(|(i, line, _)| (i, line))
        else {
            lost!("row_without_six_metre_frontage", row);
            continue;
        };
        let row_a = row[row_edge].0;
        let ru = row[(row_edge + 1) % n_row].0.sub(row_a).unit();
        let mut rv = P::new(-ru.z, ru.x);
        if centroid(&row).sub(row_a).dot(rv) < 0.0 {
            rv = rv.scale(-1.0);
        }
        let row_depth = row
            .iter()
            .map(|p| p.0.sub(row_a).dot(rv))
            .fold(0.0, f32::max);
        // Frontage width depends on the street the row faces.
        let arterial = is_arterial(row_line);
        let (min_w, max_w) = frontage_range(district, arterial);
        if cfg!(feature = "flag-passage-candidate") && !arterial && row_depth >= front_depth + 9.0 {
            for cell in cut_strip(
                row.clone(),
                ru,
                min_w + flag_passage::PASSAGE_WIDTH,
                max_w + flag_passage::PASSAGE_WIDTH,
                rng,
            ) {
                let fitted = flag_passage::fit(&cell, row_a, ru, rv, front_depth, max_depth);
                if let Some(pair) = fitted {
                    let cut = row_a.add(rv.scale(front_depth + flag_passage::PASSAGE_WIDTH * 0.5));
                    for (flag, (center, width, depth)) in [(false, pair.front), (true, pair.rear)] {
                        let ground = clip(&cell, cut, if flag { rv } else { rv.scale(-1.0) }, None);
                        let front_mid = if flag {
                            pair.passage[2]
                        } else {
                            row_a.add(ru.scale(center.sub(row_a).dot(ru)))
                        };
                        out.push(PendingLot {
                            ground: ground.iter().map(|v| v.0).collect(),
                            front_mid,
                            line: row_line,
                            lot: PlanLot {
                                x: center.x,
                                z: center.z,
                                width,
                                depth,
                                rotation: (-rv.x).atan2(-rv.z),
                                facing: rv.scale(-1.0),
                                road: usize::MAX,
                                district,
                                arterial_frontage: false,
                                flag,
                                passage: flag.then_some(pair.passage),
                                block: block_id,
                            },
                        });
                    }
                } else if let Some((center, width, depth, facing)) =
                    inscribed_rect_frame(&cell, row_a, ru, rv, front_depth)
                {
                    // Keep a legal front building when the paired passage cannot
                    // fit; do not add an inaccessible rear parcel.
                    out.push(PendingLot {
                        ground: cell.iter().map(|v| v.0).collect(),
                        front_mid: row_a.add(ru.scale(center.sub(row_a).dot(ru))),
                        line: row_line,
                        lot: PlanLot {
                            x: center.x,
                            z: center.z,
                            width,
                            depth,
                            rotation: facing.x.atan2(facing.z),
                            facing,
                            road: usize::MAX,
                            district,
                            arterial_frontage: false,
                            flag: false,
                            passage: None,
                            block: block_id,
                        },
                    });
                } else {
                    lost!("joint_flag_cell_does_not_fit", cell);
                }
            }
            continue;
        }
        // Deep rows get a rear strip of flag lots (旗竿地): narrow rear sites
        // reached by a passage beside the front lot, addressed to the same
        // street. This keeps Tokyo's fine grain without new streets.
        let mut front_strip = row.clone();
        if !arterial && row_depth >= front_depth + 9.0 {
            let depth = front_depth + rng.range(-1.5, 1.5);
            let cut = row_a.add(rv.scale(depth));
            front_strip = clip(&row, cut, rv.scale(-1.0), None);
            // Rear lots need an entrance along this finite frontage, not an
            // extrapolation beyond its end around a concave street bend.
            let rear = clip(&row, cut, rv, None);
            let rear = clip(&rear, row_a, ru, None);
            let rear = clip(&rear, row[(row_edge + 1) % n_row].0, ru.scale(-1.0), None);
            if rear.len() >= 3 {
                for lot in cut_strip(rear, ru, min_w + 1.0, max_w + 2.0, rng) {
                    if area(&lot) < 40.0 {
                        lost!("small_rear_lot", lot);
                        continue;
                    }
                    let Some((center, width, lot_depth, outward)) =
                        inscribed_rect_frame(&lot, cut, ru, rv, max_depth)
                    else {
                        lost!("rear_rectangle_does_not_fit", lot);
                        continue;
                    };
                    let front_mid = row_a.add(ru.scale(center.sub(row_a).dot(ru)));
                    out.push(PendingLot {
                        ground: lot.iter().map(|v| v.0).collect(),
                        lot: PlanLot {
                            x: center.x,
                            z: center.z,
                            width,
                            depth: lot_depth,
                            rotation: outward.x.atan2(outward.z),
                            facing: outward,
                            road: usize::MAX,
                            district,
                            arterial_frontage: false,
                            flag: true,
                            passage: None,
                            block: block_id,
                        },
                        front_mid,
                        line: row_line,
                    });
                }
            }
        }
        let pieces = cut_strip(front_strip, ru, min_w, max_w, rng);
        for lot in pieces {
            if area(&lot) < 30.0 {
                lost!("small_front_lot", lot);
                continue;
            }
            let n = lot.len();
            let has_frontage = (0..n)
                .filter_map(|i| {
                    lot[i]
                        .1
                        .map(|line| (i, line, lot[(i + 1) % n].0.sub(lot[i].0).len()))
                })
                .any(|(_, _, len)| len >= 4.0);
            if !has_frontage {
                lost!("lot_without_four_metre_frontage", lot);
                continue;
            }
            let Some((edge, line, center, width, depth, outward)) =
                fit_street_rectangle(&lot, max_depth, alternate_frontage)
            else {
                lost!("front_rectangle_does_not_fit", lot);
                continue;
            };
            let a = lot[edge].0;
            let along = lot[(edge + 1) % n].0.sub(a).unit();
            let front_mid = a.add(along.scale(center.sub(a).dot(along)));
            out.push(PendingLot {
                ground: lot.iter().map(|v| v.0).collect(),
                lot: PlanLot {
                    x: center.x,
                    z: center.z,
                    width,
                    depth,
                    rotation: outward.x.atan2(outward.z),
                    facing: outward,
                    road: usize::MAX,
                    district,
                    arterial_frontage: is_arterial(line),
                    flag: false,
                    passage: None,
                    block: block_id,
                },
                front_mid,
                line,
            });
        }
    }
}

/// Reserve a dead-end lane before lot subdivision. Convex pieces retain their
/// outer street labels; only edges alongside the actual lane gain frontage.
/// The square terminal buffer is conservative relative to radial clearance.
fn reserve_roji(inner: &Poly, start: P, end: P, line: usize) -> Vec<Poly> {
    let forward = end.sub(start).unit();
    let side = P::new(-forward.z, forward.x);
    let setback = RoadKind::Lane.lot_setback();
    let terminal = end.add(forward.scale(setback));
    let rear = clip(inner, terminal, forward, None);
    let front = clip(inner, end, forward.scale(-1.0), None);
    let cap = clip(
        &clip(inner, end, forward, None),
        terminal,
        forward.scale(-1.0),
        None,
    );
    let mut pieces = vec![rear];
    for (region, label) in [(&front, Some(line)), (&cap, None)] {
        for sign in [-1.0, 1.0] {
            let normal = side.scale(sign);
            pieces.push(clip(
                region,
                start.add(normal.scale(setback)),
                normal,
                label,
            ));
        }
    }
    pieces.retain(|piece| piece.len() >= 3 && area(piece) >= 1.0);
    pieces
}

/// Place a dead-end alley from the middle part of a block's longest local
/// street edge, running about two-thirds of the way across the block.
/// Returns the start junction (on the street) and the dead-end node.
fn roji(b: &mut Builder, inner: &Poly, rng: &mut Rng) -> Option<(usize, usize)> {
    let n = inner.len();
    let (edge, line) = (0..n)
        .filter_map(|i| inner[i].1.map(|line| (i, line)))
        .filter(|(_, line)| b.lines[*line].kind == RoadKind::Local)
        .max_by(|x, y| {
            let lx = inner[(x.0 + 1) % n].0.sub(inner[x.0].0).len();
            let ly = inner[(y.0 + 1) % n].0.sub(inner[y.0].0).len();
            lx.total_cmp(&ly)
        })?;
    let a = inner[edge].0;
    let edge_vec = inner[(edge + 1) % n].0.sub(a);
    if edge_vec.len() < 30.0 {
        return None;
    }
    let u = edge_vec.unit();
    let mut v = P::new(-u.z, u.x);
    if centroid(inner).sub(a).dot(v) < 0.0 {
        v = v.scale(-1.0);
    }
    let foot = a.add(edge_vec.scale(rng.range(0.3, 0.7)));
    // Depth available along v inside the convex block.
    let mut exit = f32::INFINITY;
    for i in 0..n {
        let (c, d) = (inner[i].0, inner[(i + 1) % n].0);
        let e = d.sub(c);
        let denom = v.cross(e);
        if denom.abs() < 1.0e-6 {
            continue;
        }
        let t = c.sub(foot).cross(e) / denom;
        let s2 = c.sub(foot).cross(v) / denom;
        if t > 0.5 && (0.0..=1.0).contains(&s2) {
            exit = exit.min(t);
        }
    }
    if !exit.is_finite() || exit < 34.0 {
        return None;
    }
    // Start on the street centreline, opposite the inset edge.
    let (la, lb) = (b.nodes[b.lines[line].a].p, b.nodes[b.lines[line].b].p);
    let t = b.line_param(line, foot).clamp(0.05, 0.95);
    let street_point = if b.lines[line].path.is_empty() {
        la.lerp(lb, t)
    } else {
        path_position(&b.lines[line].path, t)
    };
    if b.nearest_junction(line, street_point) < 12.0 {
        return None;
    }
    let start = b.split(line, street_point);
    let end = b.node(
        foot.add(v.scale(exit * rng.range(0.55, 0.7))),
        NodeKind::Junction,
    );
    Some((start, end))
}

/// Split a superblock into blocks with jittered street cuts. Cuts end on the
/// street they meet, producing T-junctions and polygonal blocks rather than a
/// continuous grid.
#[allow(clippy::too_many_arguments)]
fn split_streets(
    b: &mut Builder,
    poly: Poly,
    district: District,
    rng: &mut Rng,
    depth: u32,
    frame: P,
    blocks: &mut Vec<Poly>,
    #[cfg(test)] coarse_cell: [usize; 2],
    #[cfg(test)] trace: &mut Vec<StreetSubdivision>,
) {
    // Isolated topology experiment: retain one internal connector, then plan
    // access within each side before parcels. Do not add more through-cuts.
    #[cfg(feature = "frontage-network-candidate")]
    if district == District::StationFringe && depth > 0 {
        blocks.push(poly);
        return;
    }
    // Minimum block depth (centerline to centerline) keeps two rows of lots;
    // a cut along the long axis happens only when both halves stay that deep.
    let (min_short, max_long, jitter) = match district {
        District::StationCore => (46.0, 110.0, 0.05),
        District::StationFringe => (40.0, 100.0, 0.22),
        District::Compact => (36.0, 92.0, 0.3),
        District::Strips => (34.0, 130.0, 0.2),
        District::Waterfront => (50.0, 130.0, 0.12),
    };
    // Cuts follow the district's own street orientation (`frame`), not the
    // superblock edges, so blocks meet arterials at angles as in Tokyo.
    let (u, v, center, lu, lv) = {
        let fu = frame;
        let fv = P::new(-fu.z, fu.x);
        let (u0, u1) = extent(&poly, fu);
        let (v0, v1) = extent(&poly, fv);
        let center = fu.scale((u0 + u1) * 0.5).add(fv.scale((v0 + v1) * 0.5));
        if u1 - u0 >= v1 - v0 {
            (fu, fv, center, u1 - u0, v1 - v0)
        } else {
            (fv, fu.scale(-1.0), center, v1 - v0, u1 - u0)
        }
    };
    let min_piece = if lv > 2.0 * min_short {
        min_short
    } else {
        0.4 * max_long
    };
    let split_across_long = if lv > 2.0 * min_short {
        false
    } else if lu > max_long {
        true
    } else {
        blocks.push(poly);
        return;
    };
    if depth > 12 {
        blocks.push(poly);
        return;
    }
    // Normal of the cut line. Cutting across the long axis uses normal `u`;
    // cutting along it (to reduce the short side) uses normal `v`.
    let (base_n, extent) = if split_across_long { (u, lu) } else { (v, lv) };
    #[cfg(test)]
    let mut failed_cuts = Vec::new();
    for _attempt in 0..8 {
        // Tilt by roughly `jitter` radians using only IEEE-exact operations.
        let tilt = rng.range(-jitter, jitter);
        let n = base_n.add(P::new(-base_n.z, base_n.x).scale(tilt)).unit();
        let room = (0.5 - min_piece / extent).clamp(0.0, 0.16);
        let offset = rng.range(-room, room) * extent;
        let origin = center.add(base_n.scale(offset));
        // Find the two boundary crossings.
        let count = poly.len();
        let mut hits = Vec::new();
        for i in 0..count {
            let (p, label) = poly[i];
            let q = poly[(i + 1) % count].0;
            let sp = p.sub(origin).dot(n);
            let sq = q.sub(origin).dot(n);
            if (sp >= 0.0) != (sq >= 0.0) {
                hits.push((p.lerp(q, sp / (sp - sq)), label));
            }
        }
        if hits.len() != 2 {
            #[cfg(test)]
            failed_cuts.push(serde_json::json!({"reason":"crossing_count","count":hits.len()}));
            continue;
        }
        // Keep new junctions away from existing ones and from corners.
        let clear = hits.iter().all(|(p, label)| {
            label.is_some_and(|line| b.nearest_junction(line, *p) > 14.0)
                && poly.iter().enumerate().all(|(i, v)| {
                    if v.1 == poly[(i + poly.len() - 1) % poly.len()].1
                        && v.1.is_some_and(|l| !b.lines[l].path.is_empty())
                    {
                        return true;
                    }
                    v.0.sub(*p).len() > 14.0
                })
        });
        if !clear {
            #[cfg(test)]
            failed_cuts.push(serde_json::json!({"reason":"junction_or_corner_clearance",
                "hits":hits.iter().map(|(p,label)|serde_json::json!({
                    "point":[p.x,p.z],"line":label,
                    "junctionDistance":label.map(|line|b.nearest_junction(line,*p))
                })).collect::<Vec<_>>() }));
            continue;
        }
        let a_piece = clip(&poly, origin, n, None);
        let b_piece = clip(&poly, origin, n.scale(-1.0), None);
        if a_piece.len() < 3
            || b_piece.len() < 3
            || area(&a_piece).abs() < 500.0
            || area(&b_piece).abs() < 500.0
        {
            #[cfg(test)]
            failed_cuts.push(serde_json::json!({"reason":"small_piece",
                "areas":[area(&a_piece).abs(),area(&b_piece).abs()]}));
            continue;
        }
        // Area alone admits thin wedges from angled cuts. Preserve the intended
        // two-row depth, without rejecting a parent that was already narrower.
        #[cfg(feature = "block-proportion-candidate")]
        if !cut_preserves_block_depth(&poly, &a_piece, &b_piece, min_short) {
            continue;
        }
        // Compact superblocks retain their first cut as a local collector;
        // subsequent subdivisions serve fine-grained frontage with 4 m lanes.
        // The lattice and station districts retain their wider street hierarchy.
        let kind = if (district == District::Compact && depth > 0)
            || (district == District::Strips && split_across_long && rng.unit() < 0.6)
        {
            RoadKind::Lane
        } else {
            RoadKind::Local
        };
        #[cfg(test)]
        if std::env::var_os("PLAN_PREVIEW_STREET_FRONTAGE").is_some() {
            street_frontage_preview::trace(
                &b, &poly, &a_piece, &b_piece, district, kind, depth, center, base_n, extent,
                min_piece, jitter,
            );
        }
        #[cfg(feature = "street-placement-candidate")]
        let (a_piece, b_piece, hits) = street_frontage_preview::choose(
            &b,
            &poly,
            &a_piece,
            &b_piece,
            [hits[0].1.unwrap(), hits[1].1.unwrap()],
            district,
            kind,
            center,
            base_n,
            extent,
            min_piece,
            jitter,
        )
        .unwrap_or((a_piece, b_piece, hits));
        #[cfg(test)]
        let input_junctions = poly
            .iter()
            .map(|(_, label)| {
                let mut points = label.map_or_else(Vec::new, |line| {
                    let l = &b.lines[line];
                    std::iter::once(l.a)
                        .chain(std::iter::once(l.b))
                        .chain(l.splits.iter().map(|s| s.1))
                        .map(|n| [b.nodes[n].p.x, b.nodes[n].p.z])
                        .collect::<Vec<_>>()
                });
                points.sort_by(|a, b| a[0].total_cmp(&b[0]).then_with(|| a[1].total_cmp(&b[1])));
                points
            })
            .collect();
        let na = b.split(hits[0].1.unwrap(), hits[0].0);
        let nb = b.split(hits[1].1.unwrap(), hits[1].0);
        let line = b.line(na, nb, kind);
        #[cfg(test)]
        trace.push(StreetSubdivision {
            coarse_cell,
            input_junctions,
            depth,
            district: format!("{district:?}"),
            parent: poly.iter().map(|(p, _)| [p.x, p.z]).collect(),
            line,
            host_lines: [hits[0].1.unwrap(), hits[1].1.unwrap()],
            endpoints: [[hits[0].0.x, hits[0].0.z], [hits[1].0.x, hits[1].0.z]],
            kind: kind.as_str().into(),
            roads: Vec::new(),
        });
        let relabel = |piece: Poly| -> Poly {
            piece
                .into_iter()
                .map(|(p, label)| (p, label.or(Some(line))))
                .collect()
        };
        split_streets(
            b,
            relabel(a_piece),
            district,
            rng,
            depth + 1,
            frame,
            blocks,
            #[cfg(test)]
            coarse_cell,
            #[cfg(test)]
            trace,
        );
        split_streets(
            b,
            relabel(b_piece),
            district,
            rng,
            depth + 1,
            frame,
            blocks,
            #[cfg(test)]
            coarse_cell,
            #[cfg(test)]
            trace,
        );
        return;
    }
    #[cfg(test)]
    if std::env::var_os("CITY_TRACE_FAILED_CUTS").is_some() {
        eprintln!(
            "FAILED_STREET_CUT {}",
            serde_json::json!({"depth":depth,
            "district":format!("{district:?}"),"long":lu,"short":lv,
            "points":poly.iter().map(|(p,_)|[p.x,p.z]).collect::<Vec<_>>(),
            "attempts":failed_cuts})
        );
    }
    blocks.push(poly);
}

#[cfg(any(test, feature = "block-proportion-candidate"))]
fn block_caliper_width(poly: &Poly) -> f32 {
    (0..poly.len())
        .filter_map(|i| {
            let d = poly[(i + 1) % poly.len()].0.sub(poly[i].0);
            if d.len() < 0.001 {
                return None;
            }
            let normal = P::new(-d.z, d.x).unit();
            let (lo, hi) = extent(poly, normal);
            Some(hi - lo)
        })
        .fold(f32::INFINITY, f32::min)
}

#[cfg(any(test, feature = "block-proportion-candidate"))]
fn cut_preserves_block_depth(parent: &Poly, a: &Poly, b: &Poly, minimum: f32) -> bool {
    let required = block_caliper_width(parent).min(minimum);
    block_caliper_width(a) + 0.05 >= required && block_caliper_width(b) + 0.05 >= required
}

struct LatticeLine {
    kind: LineKind,
}

pub fn generate(geo: &Geography) -> MasterPlan {
    generate_layout(geo, true, !cfg!(feature = "single-expressway-candidate"))
}

#[cfg(any(test, feature = "test-support"))]
pub fn curved_rail_candidate(geo: &Geography) -> MasterPlan {
    generate_impl(geo, true)
}

// Straight mode remains available only for controlled geometry comparisons.
fn generate_impl(geo: &Geography, curved_rail: bool) -> MasterPlan {
    generate_layout(geo, curved_rail, true)
}

// Keep the single regional corridor candidate opt-in until visual and
// runtime acceptance. Both modes reserve their real geometry before subdivision.
fn generate_layout(geo: &Geography, curved_rail: bool, crossing_expressway: bool) -> MasterPlan {
    generate_layout_with_frontage(
        geo,
        curved_rail,
        crossing_expressway,
        cfg!(feature = "frontage-fit-candidate"),
    )
}

fn generate_layout_with_frontage(
    geo: &Geography,
    curved_rail: bool,
    crossing_expressway: bool,
    alternate_frontage: bool,
) -> MasterPlan {
    let mut rng = Rng::new(geo.seed);
    let river = &geo.river;
    let l = PLAN_EXTENT;
    let banks = [
        crate::river_course::bank_alignment(river, -BANK_OFFSET),
        crate::river_course::bank_alignment(river, BANK_OFFSET),
    ];

    // Coarse collector stations are independent of river sample density.
    // Increasing curve resolution must not multiply the arterial lattice.
    let mut rows: Vec<(f32, LatticeLine)> = vec![(
        -l,
        LatticeLine {
            kind: LineKind::Ring,
        },
    )];
    for z in [-190.0, 20.0, 210.0] {
        rows.push((
            z,
            LatticeLine {
                kind: LineKind::Collector,
            },
        ));
    }
    rows.push((
        l,
        LatticeLine {
            kind: LineKind::Ring,
        },
    ));
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (band_lo, band_hi) = rows
        .windows(2)
        .map(|w| (w[0].0, w[1].0))
        .filter(|(lo, hi)| *lo > -l + 1.0 && *hi < l - 1.0)
        .max_by(|x, y| ((x.1 - x.0) - x.0.abs() * 0.1).total_cmp(&((y.1 - y.0) - y.0.abs() * 0.1)))
        .unwrap_or((20.0, 210.0));
    let rail_z = (band_lo + band_hi) * 0.5 + rng.range(-12.0, 12.0);
    rows.push((
        rail_z - RAIL_BAND_HALF,
        LatticeLine {
            kind: LineKind::RailFrontage,
        },
    ));
    rows.push((
        rail_z + RAIL_BAND_HALF,
        LatticeLine {
            kind: LineKind::RailFrontage,
        },
    ));
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    // Two arterial rows: the gateway row south of the rail corridor and the
    // row north of it that carries the expressway.
    let promote = |rows: &mut Vec<(f32, LatticeLine)>, pick: &dyn Fn(f32) -> f32| {
        if let Some(index) = rows
            .iter()
            .enumerate()
            .filter(|(_, (_, line))| line.kind == LineKind::Collector)
            .min_by(|x, y| pick(x.1 .0).total_cmp(&pick(y.1 .0)))
            .map(|(index, _)| index)
        {
            rows[index].1.kind = LineKind::Arterial;
        }
    };
    promote(&mut rows, &|z| (z - rail_z + 60.0).abs());
    promote(&mut rows, &|z| {
        if z > rail_z {
            z - rail_z
        } else {
            f32::INFINITY
        }
    });

    let west_mid = 0.5 + rng.range(-0.08, 0.08);
    let east_a = 1.0 / 3.0 + rng.range(-0.05, 0.05);
    let east_b = 2.0 / 3.0 + rng.range(-0.05, 0.05);
    let bank_w = |z: f32| river_x(&banks[0], z);
    let bank_e = |z: f32| river_x(&banks[1], z);
    let columns: Vec<(Box<dyn Fn(f32) -> f32>, LineKind)> = vec![
        (Box::new(move |_| -l), LineKind::Ring),
        (
            Box::new(move |z| -l + (bank_w(z) + l) * west_mid),
            LineKind::Collector,
        ),
        // Riverbank streets collect neighborhood traffic. Regional through
        // movement belongs to the crossing rows and inland arterial column;
        // two 16 m bank arterials overwhelmed the waterfront frontage.
        (Box::new(bank_w), LineKind::Collector),
        (Box::new(bank_e), LineKind::Collector),
        (
            Box::new(move |z| bank_e(z) + (l - bank_e(z)) * east_a),
            LineKind::Arterial,
        ),
        (
            Box::new(move |z| bank_e(z) + (l - bank_e(z)) * east_b),
            LineKind::Collector,
        ),
        (Box::new(move |_| l), LineKind::Ring),
    ];
    const RIVER_COL: usize = 2;

    let mut b = Builder {
        nodes: Vec::new(),
        index: BTreeMap::new(),
        roads: Vec::new(),
        lines: Vec::new(),
        line_lookup: BTreeMap::new(),
    };
    let rail_band_row = rows
        .iter()
        .position(|(z, _)| (*z - (rail_z - RAIL_BAND_HALF)).abs() < 0.01)
        .unwrap();
    // Perturb interior arterial junctions so superblocks are irregular
    // quadrilaterals rather than a rectangular lattice. Bank arterials, rail
    // frontages and the outer ring keep their constraints.
    let base: Vec<Vec<P>> = rows
        .iter()
        .map(|(z, _)| columns.iter().map(|(f, _)| P::new(f(*z), *z)).collect())
        .collect();
    let mut jittered = base.clone();
    // Seeded slopes turn interior arterials diagonal, as Tokyo's main roads
    // cut across the local fabric. Bank roads, the rail corridor frontage and
    // the outer ring keep their positions.
    let row_slope: Vec<f32> = rows.iter().map(|_| rng.range(-0.3, 0.3)).collect();
    let col_slope: Vec<f32> = columns.iter().map(|_| rng.range(-0.28, 0.28)).collect();
    for (r, (z, line)) in rows.iter().enumerate() {
        for (c, (_, col_kind)) in columns.iter().enumerate() {
            let bank = c == RIVER_COL || c == RIVER_COL + 1;
            let ring_col = *col_kind == LineKind::Ring;
            if bank || ring_col {
                continue;
            }
            let p = base[r][c];
            if !matches!(line.kind, LineKind::Ring | LineKind::RailFrontage) {
                let anchor = if c < RIVER_COL {
                    base[r][RIVER_COL].x
                } else {
                    base[r][RIVER_COL + 1].x
                };
                jittered[r][c].z = *z + row_slope[r] * (p.x - anchor) * 0.5;
            }
            if line.kind != LineKind::RailFrontage {
                jittered[r][c].x = p.x + col_slope[c] * p.z * 0.5;
            }
        }
    }
    for (r, (_, line)) in rows.iter().enumerate() {
        for (c, (_, col_kind)) in columns.iter().enumerate() {
            let ring_row = line.kind == LineKind::Ring;
            let rail_row = line.kind == LineKind::RailFrontage;
            let bank = c == RIVER_COL || c == RIVER_COL + 1;
            if matches!(*col_kind, LineKind::Arterial | LineKind::Collector) && !bank {
                jittered[r][c].x += rng.range(-30.0, 30.0);
            }
            if !ring_row && !rail_row && !bank {
                jittered[r][c].z += rng.range(-26.0, 26.0);
            }
        }
    }
    // Bent arterials must still bound convex superblocks with no sharp
    // corners; pull offending junctions back toward the unbent lattice.
    let convex = |q: [P; 4]| -> bool {
        (0..4).all(|i| {
            let a = q[(i + 1) % 4].sub(q[i]);
            let b2 = q[(i + 2) % 4].sub(q[(i + 1) % 4]);
            // sin of the turn angle; require turns of at least ~30 degrees.
            a.cross(b2) > 0.5 * a.len() * b2.len()
        })
    };
    for _ in 0..12 {
        let mut changed = false;
        for r in 0..rows.len() - 1 {
            for c in 0..columns.len() - 1 {
                let corners = [(r, c), (r, c + 1), (r + 1, c + 1), (r + 1, c)];
                if convex(corners.map(|(rr, cc)| jittered[rr][cc])) {
                    continue;
                }
                for (rr, cc) in corners {
                    jittered[rr][cc] = jittered[rr][cc].lerp(base[rr][cc], 0.5);
                }
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let grid: Vec<Vec<usize>> = jittered
        .iter()
        .map(|row| row.iter().map(|p| b.node(*p, NodeKind::Junction)).collect())
        .collect();

    let mut row_lines = vec![vec![None; columns.len() - 1]; rows.len()];
    for (r, (_, line)) in rows.iter().enumerate() {
        for c in 0..columns.len() - 1 {
            if c == RIVER_COL {
                if matches!(line.kind, LineKind::Arterial | LineKind::Collector) {
                    row_lines[r][c] = Some(b.line(grid[r][c], grid[r][c + 1], RoadKind::Bridge));
                }
                continue;
            }
            row_lines[r][c] = Some(b.line(grid[r][c], grid[r][c + 1], line.kind.road()));
        }
    }
    let mut col_lines = vec![vec![0usize; columns.len()]; rows.len() - 1];
    for r in 0..rows.len() - 1 {
        for (c, (_, kind)) in columns.iter().enumerate() {
            col_lines[r][c] = b.line(grid[r][c], grid[r + 1][c], kind.road());
            if c == RIVER_COL || c == RIVER_COL + 1 {
                let a = b.nodes[grid[r][c]].p;
                let end = b.nodes[grid[r + 1][c]].p;
                let mut path = vec![a];
                path.extend(
                    banks[c - RIVER_COL]
                        .iter()
                        .copied()
                        .filter(|p| p.z > a.z && p.z < end.z),
                );
                path.push(end);
                b.lines[col_lines[r][c]].path = path;
            }
        }
    }

    let mut station_x = rng.range(110.0, 190.0);
    let rail_col_xs: Vec<f32> = (0..columns.len())
        .map(|c| {
            let lo = b.nodes[grid[rail_band_row][c]].p;
            let hi = b.nodes[grid[rail_band_row + 1][c]].p;
            (lo.x + hi.x) * 0.5
        })
        .collect();
    for _ in 0..8 {
        if rail_col_xs.iter().all(|x| (x - station_x).abs() > 22.0) {
            break;
        }
        station_x += 13.0;
    }
    if curved_rail {
        station_x = crate::rail_alignment::Alignment::station_x(station_x, EDGE, &rail_col_xs);
    }
    let station_p = P::new(station_x, rail_z);
    let rail_alignment =
        curved_rail.then(|| crate::rail_alignment::Alignment::new(station_p, EDGE, geo.seed));
    if let Some(alignment) = &rail_alignment {
        // Move the two frontage rows BEFORE block subdivision. Solve their
        // bank intersections against the actual bank polylines, then rebuild
        // adjoining bank paths using those shared junctions.
        for (r, offset) in [
            (rail_band_row, -RAIL_BAND_HALF),
            (rail_band_row + 1, RAIL_BAND_HALF),
        ] {
            for c in 0..columns.len() {
                let node = grid[r][c];
                let mut x = b.nodes[node].p.x;
                if c == RIVER_COL || c == RIVER_COL + 1 {
                    for _ in 0..24 {
                        let z = alignment.boundary_at_x(x, offset).z;
                        x = river_x(&banks[c - RIVER_COL], z);
                    }
                }
                b.nodes[node].p = alignment.boundary_at_x(x, offset);
            }
            for c in 0..columns.len() - 1 {
                let Some(line) = row_lines[r][c] else {
                    continue;
                };
                let a = b.nodes[grid[r][c]].p;
                let end = b.nodes[grid[r][c + 1]].p;
                let mut path = vec![a];
                // Coarser street sections retain useful frontage lengths;
                // the rail's arc-length sampling remains two metres.
                for (_, sample) in alignment.samples().iter().step_by(6) {
                    if sample.x > a.x + 3.0 && sample.x < end.x - 3.0 {
                        path.push(alignment.boundary_at_x(sample.x, offset));
                    }
                }
                path.push(end);
                b.lines[line].path = path;
            }
        }
        for r in 0..rows.len() - 1 {
            for c in [RIVER_COL, RIVER_COL + 1] {
                let a = b.nodes[grid[r][c]].p;
                let end = b.nodes[grid[r + 1][c]].p;
                let mut path = vec![a];
                path.extend(
                    banks[c - RIVER_COL]
                        .iter()
                        .copied()
                        .filter(|p| p.z > a.z && p.z < end.z),
                );
                path.push(end);
                b.lines[col_lines[r][c]].path = path;
            }
        }
        b.index = b
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                (
                    (
                        (n.p.x * 100.0).round() as i64,
                        (n.p.z * 100.0).round() as i64,
                    ),
                    i,
                )
            })
            .collect();
    }
    let station_dist = |p: P| p.sub(station_p).len();

    let mut pending: Vec<PendingLot> = Vec::new();
    #[cfg(test)]
    let mut rejected_lot_ground = Vec::new();
    #[cfg(test)]
    let mut block_land_budget = Vec::new();
    #[cfg(test)]
    let mut street_land_budget = Vec::new();
    #[cfg(test)]
    let mut partition_rejected_ground = Vec::new();
    let mut block_outlines: Vec<Vec<P>> = Vec::new();
    let mut block_id = 0u32;
    let mut all_blocks: Vec<(Poly, District, u32)> = Vec::new();
    #[cfg(test)]
    let mut street_subdivisions = Vec::new();
    let mut parent_count = 0u32;
    let mut block_parents = Vec::new();
    #[cfg(feature = "parent-passage-candidate")]
    let mut parent_ground = Vec::new();
    for r in 0..rows.len() - 1 {
        if r == rail_band_row {
            continue;
        }
        for c in 0..columns.len() - 1 {
            if c == RIVER_COL {
                continue;
            }
            let corners = [
                grid[r][c],
                grid[r][c + 1],
                grid[r + 1][c + 1],
                grid[r + 1][c],
            ];
            let quad: [P; 4] = corners.map(|n| b.nodes[n].p);
            // Protected land keeps only its own footprint open: streets and
            // lots touching it are filtered after subdivision.
            let poly: Poly = ccw(vec![
                (quad[0], row_lines[r][c]),
                (quad[1], Some(col_lines[r][c + 1])),
                (quad[2], row_lines[r + 1][c]),
                (quad[3], Some(col_lines[r][c])),
            ]);
            let poly = {
                let mut expanded = Vec::new();
                for (i, &(p, label)) in poly.iter().enumerate() {
                    expanded.push((p, label));
                    if let Some(line) = label {
                        let path = &b.lines[line].path;
                        if path.len() > 2 {
                            let forward = p.sub(path[0]).len() < p.sub(*path.last().unwrap()).len();
                            let points: Vec<_> = path[1..path.len() - 1].iter().copied().collect();
                            if forward {
                                expanded.extend(points.into_iter().map(|p| (p, label)));
                            } else {
                                expanded.extend(points.into_iter().rev().map(|p| (p, label)));
                            }
                        }
                    }
                }
                expanded
            };
            // Coarse-cell stream: topology changes consume only this parent's
            // random choices, never those of subsequent neighborhoods.
            let mut rng = Rng::new(
                geo.seed
                    ^ 0x6672_6e74
                    ^ (r as u32).wrapping_mul(0x9e37_79b9)
                    ^ (c as u32).wrapping_mul(0x85eb_ca6b),
            );
            let mid = centroid(&poly);
            let distance = station_dist(mid);
            let waterfront = c + 1 == RIVER_COL || c == RIVER_COL + 1;
            let district = if distance < 150.0 {
                District::StationCore
            } else if distance < 260.0 {
                District::StationFringe
            } else if waterfront && rng.unit() < 0.5 {
                District::Waterfront
            } else if rng.unit() < 0.3 {
                District::Strips
            } else {
                District::Compact
            };
            let mut blocks = Vec::new();
            // District street orientation: the superblock's long axis turned
            // by a seeded angle. Old compact districts turn furthest.
            let turn = match district {
                District::StationCore => 0.08,
                District::Waterfront => 0.18,
                District::StationFringe => 0.35,
                District::Strips => 0.45,
                District::Compact => 0.7,
            };
            let (axis, _, _, _, _) = obb(&poly);
            let frame = axis
                .add(P::new(-axis.z, axis.x).scale(rng.range(-turn, turn)))
                .unit();
            split_streets(
                &mut b,
                poly,
                district,
                &mut rng,
                0,
                frame,
                &mut blocks,
                #[cfg(test)]
                [r, c],
                #[cfg(test)]
                &mut street_subdivisions,
            );
            for block in blocks {
                all_blocks.extend(
                    convex_pieces(block)
                        .into_iter()
                        .map(|p| (p, district, parent_count)),
                );
                parent_count += 1;
            }
        }
    }

    let station = b.node(station_p, NodeKind::Station);
    let south_col = rail_col_xs
        .windows(2)
        .position(|w| w[0] <= station_x && station_x <= w[1])
        .unwrap_or(4);
    let access_line = row_lines[rail_band_row][south_col].unwrap();
    let access_point = {
        let la = b.nodes[b.lines[access_line].a].p;
        let lb = b.nodes[b.lines[access_line].b].p;
        let t = ((station_x - la.x) / (lb.x - la.x)).clamp(0.05, 0.95);
        if b.lines[access_line].path.is_empty() {
            la.lerp(lb, t)
        } else {
            // Station approaches are straight even when the wider frontage
            // line bends. Do not split on its endpoint chord.
            let path = &b.lines[access_line].path;
            let pair = path
                .windows(2)
                .find(|pair| pair[0].x <= station_x && station_x <= pair[1].x)
                .unwrap();
            pair[0].lerp(pair[1], (station_x - pair[0].x) / (pair[1].x - pair[0].x))
        }
    };
    let access_node = b.split(access_line, access_point);

    // Elevated expressway on land, Shuto-style: it rides above one arterial
    // row and one arterial column and crosses the river only on the row's
    // bridge, never running along the water. Elevated nodes are separate from
    // street junctions, so the expressway is grade-separated from the grid.
    let mut expressway = Vec::new();
    let mut row_elevated: BTreeMap<usize, usize> = BTreeMap::new();
    let mut row_lift = |b: &mut Builder, street_node: usize| -> usize {
        *row_elevated.entry(street_node).or_insert_with(|| {
            b.nodes.push(PlanNode {
                p: b.nodes[street_node].p,
                kind: NodeKind::Elevated,
            });
            b.nodes.len() - 1
        })
    };
    let mut edge_node = |b: &mut Builder, p: P| -> usize {
        b.nodes.push(PlanNode {
            p,
            kind: NodeKind::Elevated,
        });
        b.nodes.len() - 1
    };
    let expressway_row = rows
        .iter()
        .position(|(z, line)| line.kind == LineKind::Arterial && *z > rail_z)
        .or_else(|| {
            rows.iter()
                .position(|(_, line)| line.kind == LineKind::Arterial)
        })
        .unwrap();
    // Continue the last street bearing to the regional boundary. Forcing a
    // short outer segment back to an axis created tight hooks (seed 1234).
    let boundary = |outer: P, inner: P, edge: f32, along_x: bool| {
        let delta = outer.sub(inner);
        if along_x {
            P::new(
                edge,
                (outer.z + delta.z * (edge - outer.x) / delta.x).clamp(-EDGE, EDGE),
            )
        } else {
            P::new(
                (outer.x + delta.x * (edge - outer.z) / delta.z).clamp(-EDGE, EDGE),
                edge,
            )
        }
    };
    let east = boundary(
        b.nodes[grid[expressway_row][columns.len() - 1]].p,
        b.nodes[grid[expressway_row][columns.len() - 2]].p,
        EDGE,
        true,
    );
    let mut route: Vec<usize> = vec![edge_node(&mut b, east)];
    for c in (0..columns.len()).rev() {
        route.push(row_lift(&mut b, grid[expressway_row][c]));
    }
    let west = boundary(
        b.nodes[grid[expressway_row][0]].p,
        b.nodes[grid[expressway_row][1]].p,
        -EDGE,
        true,
    );
    route.push(edge_node(&mut b, west));
    for pair in route.windows(2) {
        expressway.push(b.road(pair[0], pair[1], RoadKind::Expressway));
    }
    drop(row_lift);
    let mut spur = Vec::new();
    if crossing_expressway {
        // Independent north-south route above the first eastern interior arterial column.
        let column = RIVER_COL + 2;
        let bottom = boundary(
            b.nodes[grid[0][column]].p,
            b.nodes[grid[1][column]].p,
            -EDGE,
            false,
        );
        spur.push(edge_node(&mut b, bottom));
        let mut spur_elevated: BTreeMap<usize, usize> = BTreeMap::new();
        let mut spur_lift = |b: &mut Builder, street_node: usize| -> usize {
            *spur_elevated.entry(street_node).or_insert_with(|| {
                b.nodes.push(PlanNode {
                    p: b.nodes[street_node].p,
                    kind: NodeKind::Elevated,
                });
                b.nodes.len() - 1
            })
        };
        for r in 0..rows.len() {
            spur.push(spur_lift(&mut b, grid[r][column]));
        }
        let top = boundary(
            b.nodes[grid[rows.len() - 1][column]].p,
            b.nodes[grid[rows.len() - 2][column]].p,
            EDGE,
            false,
        );
        spur.push(edge_node(&mut b, top));
        for pair in spur.windows(2) {
            let road_index = b.road(pair[0], pair[1], RoadKind::Expressway);
            b.roads[road_index].deck_height = 20.0;
            expressway.push(road_index);
        }
    }

    // Elevated corridors need a broader bearing than noisy street junctions.
    // Only their independent nodes move; street access and blocks stay fixed.
    for (route_nodes, along_x) in [(&route, true), (&spur, false)] {
        if route_nodes.len() < 2 {
            continue;
        }
        let mut points: Vec<P> = route_nodes.iter().map(|&i| b.nodes[i].p).collect();
        crate::road_curve::fair_route(&mut points, along_x);
        for (&index, point) in route_nodes.iter().zip(points) {
            b.nodes[index].p = point;
        }
    }

    // Fix the complete route before any lots are admitted or sections realized.
    // A growing route must not change the curve of an already-built section.
    for &index in &expressway {
        let road = b.roads[index];
        let neighbor = |node: usize| -> Option<P> {
            let connected: Vec<_> = expressway
                .iter()
                .copied()
                .filter(|&i| {
                    let other = b.roads[i];
                    (other.a == node || other.b == node) && other.deck_height == road.deck_height
                })
                .collect();
            if connected.len() != 2 {
                return None;
            }
            let other = b.roads[*connected.iter().find(|&&i| i != index)?];
            Some(b.nodes[if other.a == node { other.b } else { other.a }].p)
        };
        let curve = crate::road_curve::controls(
            b.nodes[road.a].p,
            b.nodes[road.b].p,
            neighbor(road.a),
            neighbor(road.b),
        );
        b.roads[index].curve = Some(curve);
    }
    let elevated_clearance: Vec<_> = expressway
        .iter()
        .flat_map(|&index| {
            let road = b.roads[index];
            let (a, end) = (b.nodes[road.a].p, b.nodes[road.b].p);
            let c = road.curve.unwrap();
            // A cubic's distance from its sampled chord is bounded by M*h²/8,
            // where M bounds the second derivative. Include that error in clearance.
            let second = a
                .sub(c[0].scale(2.0))
                .add(c[1])
                .len()
                .max(c[0].sub(c[1].scale(2.0)).add(end).len())
                * 6.0;
            let clearance = 10.8 + second / (8.0 * 32.0 * 32.0);
            (0..32).map(move |i| {
                (
                    crate::road_curve::sample(a, c, end, i as f32 / 32.0),
                    crate::road_curve::sample(a, c, end, (i + 1) as f32 / 32.0),
                    clearance,
                )
            })
        })
        .collect();
    // Lots, after every street exists so setbacks see final kinds.
    for (block, district, parent) in &all_blocks {
        let mut rng = block_rng(geo.seed, block);
        let line_kind = |label: Option<usize>| label.map(|l| b.lines[l].kind);
        let inner = inset(block, |label| {
            line_kind(label).map_or(0.0, |k| k.lot_setback())
        });
        #[cfg(test)]
        let after_setback_area = area(&inner);
        let inner = trim_curve_frontage(inner, &b);
        #[cfg(test)]
        street_land_budget.push(StreetLandBudget {
            district: format!("{district:?}"),
            raw_area: area(block),
            points: block.iter().map(|(p, _)| [p.x, p.z]).collect(),
            edge_kinds: block
                .iter()
                .map(|(_, label)| line_kind(*label).map(|k| k.as_str().into()))
                .collect(),
            after_setback_area,
            after_curve_trim_area: area(&inner),
        });
        if inner.len() < 3 {
            continue;
        }
        // Dead-end roji: a 4 m alley from a local street into a deep block.
        let roji_chance = match district {
            District::Compact => 0.55,
            District::Strips => 0.45,
            District::StationFringe => 0.3,
            District::Waterfront => 0.2,
            District::StationCore => 0.1,
        };
        let roji_roll = rng.unit();
        let required_frontage_lane =
            cfg!(feature = "frontage-network-candidate") && *district == District::StationFringe;
        let pieces = if required_frontage_lane || roji_roll < roji_chance {
            if let Some((start, end)) = roji(&mut b, &inner, &mut rng) {
                let line = b.line(start, end, RoadKind::Lane);
                reserve_roji(&inner, b.nodes[start].p, b.nodes[end].p, line)
            } else {
                vec![inner.clone()]
            }
        } else {
            vec![inner.clone()]
        };
        let is_arterial = |line: usize| b.lines[line].kind == RoadKind::Arterial;
        #[cfg(test)]
        let mut budget = BlockLandBudget {
            block: block_id,
            inset_area: area(&inner),
            after_roji_area: pieces.iter().map(area).sum(),
            after_elevated_area: 0.0,
            pending_area: 0.0,
        };
        #[cfg(test)]
        let pending_start = pending.len();
        for piece in pieces {
            for reserved in reserve_elevated_corridor(piece, &elevated_clearance) {
                #[cfg(feature = "parent-passage-candidate")]
                parent_ground.push(parent_passage::Piece {
                    parent: *parent,
                    block: block_id,
                    district: *district,
                    poly: reserved.clone(),
                });
                #[cfg(test)]
                {
                    budget.after_elevated_area += area(&reserved);
                }
                partition_block(
                    &reserved,
                    *district,
                    block_id,
                    &is_arterial,
                    &mut rng,
                    &mut pending,
                    alternate_frontage,
                    #[cfg(test)]
                    &mut partition_rejected_ground,
                );
            }
        }
        #[cfg(test)]
        {
            budget.pending_area = pending[pending_start..]
                .iter()
                .map(|lot| area(&lot.ground.iter().map(|&p| (p, None)).collect()))
                .sum();
            block_land_budget.push(budget);
        }
        block_outlines.push(inner.iter().map(|v| v.0).collect());
        block_parents.push(*parent);
        block_id += 1;
    }

    b.resolve_lines(geo.protected);
    b.road(access_node, station, RoadKind::Local);

    let gateway_row = rows
        .iter()
        .enumerate()
        .filter(|(_, (_, line))| line.kind == LineKind::Arterial)
        .min_by(|x, y| {
            (x.1 .0 - rail_z + 60.0)
                .abs()
                .total_cmp(&(y.1 .0 - rail_z + 60.0).abs())
        })
        .map(|(r, _)| r)
        .unwrap();
    let east = grid[gateway_row][columns.len() - 1];
    let gateway = b.node(P::new(EDGE, b.nodes[east].p.z), NodeKind::Gateway);
    let gateway_road = b.road(gateway, east, RoadKind::Arterial);
    // Founding opens the gateway arterial west to the east-bank junction, so
    // the first settlement grows on the waterfront like the original town.
    let mut founding_roads = vec![gateway_road];
    for c in (RIVER_COL + 1..columns.len() - 1).rev() {
        if let Some(line) = row_lines[gateway_row][c] {
            let mut segments = b.lines[line].roads.clone();
            segments.reverse();
            founding_roads.extend(segments.iter().map(|s| s.2));
        }
    }
    let founding_market = grid[gateway_row][RIVER_COL + 1];

    // Founding also opens the shortest street path from the market to the
    // station, so rail service can start while the first residents arrive.
    {
        let mut adjacency: BTreeMap<usize, Vec<(usize, usize, f32)>> = BTreeMap::new();
        for (index, road) in b.roads.iter().enumerate() {
            if !road.kind.is_street() {
                continue;
            }
            let length = b.nodes[road.a].p.sub(b.nodes[road.b].p).len();
            adjacency
                .entry(road.a)
                .or_default()
                .push((road.b, index, length));
            adjacency
                .entry(road.b)
                .or_default()
                .push((road.a, index, length));
        }
        let mut dist: BTreeMap<usize, f32> = BTreeMap::new();
        let mut prev: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
        let mut open: Vec<(f32, usize)> = vec![(0.0, founding_market)];
        dist.insert(founding_market, 0.0);
        while let Some(position) = (0..open.len()).min_by(|x, y| open[*x].0.total_cmp(&open[*y].0))
        {
            let (d, node) = open.swap_remove(position);
            if node == station {
                break;
            }
            if d > *dist.get(&node).unwrap_or(&f32::INFINITY) {
                continue;
            }
            for &(next, road, length) in adjacency.get(&node).into_iter().flatten() {
                let candidate = d + length;
                if candidate < *dist.get(&next).unwrap_or(&f32::INFINITY) {
                    dist.insert(next, candidate);
                    prev.insert(next, (node, road));
                    open.push((candidate, next));
                }
            }
        }
        let mut path = Vec::new();
        let mut node = station;
        while let Some(&(from, road)) = prev.get(&node) {
            path.push(road);
            node = from;
        }
        path.reverse();
        for road in path {
            if !founding_roads.contains(&road) {
                founding_roads.push(road);
            }
        }
    }

    let rail_gateway = b.node(
        rail_alignment
            .as_ref()
            .map_or(P::new(EDGE, rail_z), |a| a.at_x(EDGE).0),
        NodeKind::RailGateway,
    );
    let rail_terminus = b.node(
        rail_alignment
            .as_ref()
            .map_or(P::new(-EDGE, rail_z), |a| a.at_x(-EDGE).0),
        NodeKind::RailTerminus,
    );
    let rail_service = b.road(station, rail_gateway, RoadKind::Rail);
    let rail_extension = b.road(station, rail_terminus, RoadKind::Rail);

    #[cfg(test)]
    for cut in &mut street_subdivisions {
        cut.roads = b.lines[cut.line].roads.iter().map(|s| s.2).collect();
    }
    let Builder {
        nodes,
        roads,
        lines,
        ..
    } = b;
    let mut lots = Vec::new();
    let mut lot_ground = Vec::new();
    for pending_lot in pending {
        let line = &lines[pending_lot.line];
        let (pa, pb) = (nodes[line.a].p, nodes[line.b].p);
        let d = pb.sub(pa);
        let t = pending_lot.front_mid.sub(pa).dot(d) / d.dot(d);
        let frontage = if line.path.is_empty() {
            line.roads
                .iter()
                .find(|(t0, t1, _)| t >= *t0 - 1.0e-4 && t <= *t1 + 1.0e-4)
                .map(|s| s.2)
        } else {
            // An inset/rear lot can be closer to the next bend than to the
            // segment its frontage actually follows. Match the frontage
            // bearing before selecting its nearest resolved road piece.
            line.roads
                .iter()
                .map(|s| s.2)
                .filter(|&index| {
                    let r = roads[index];
                    let a = nodes[r.a].p;
                    let d = nodes[r.b].p.sub(a);
                    let t = pending_lot.front_mid.sub(a).dot(d) / d.dot(d);
                    // A parallel segment beyond the frontage endpoint is not access.
                    // This matters for large rows near a sampled curved street.
                    (-1.0e-4..=1.0001).contains(&t)
                        && d.unit().dot(pending_lot.lot.facing).abs() < 0.001
                })
                .min_by(|&x, &y| {
                    let distance = |index: usize| {
                        let r = roads[index];
                        point_segment_distance(pending_lot.front_mid, nodes[r.a].p, nodes[r.b].p)
                    };
                    distance(x).total_cmp(&distance(y))
                })
        };
        let Some(road) = frontage else {
            #[cfg(test)]
            rejected_lot_ground.push(("unresolved_frontage", pending_lot.ground));
            continue;
        };
        let mut lot = pending_lot.lot;
        lot.road = road;
        if let Some(reason) = lot_clearance_rejection(
            &lot,
            geo,
            rail_alignment.as_ref(),
            rail_z,
            &nodes,
            &roads,
            &elevated_clearance,
            None,
        ) {
            #[cfg(test)]
            rejected_lot_ground.push((reason, pending_lot.ground));
            #[cfg(not(test))]
            let _ = reason;
            continue;
        }
        lot_ground.push(pending_lot.ground);
        lots.push(lot);
        if lots.len() >= MAX_PLAN_LOTS {
            break;
        }
    }

    #[cfg(feature = "parent-passage-candidate")]
    parent_passage::append(
        &parent_ground,
        &lines,
        &nodes,
        &roads,
        &mut lots,
        &mut lot_ground,
        |lot, access_road| {
            lot_clearance_rejection(
                lot,
                geo,
                rail_alignment.as_ref(),
                rail_z,
                &nodes,
                &roads,
                &elevated_clearance,
                access_road,
            )
            .is_none()
        },
    );

    MasterPlan {
        rail_alignment,
        #[cfg(test)]
        block_land_budget,
        #[cfg(test)]
        street_land_budget,
        #[cfg(test)]
        street_subdivisions,
        #[cfg(test)]
        partition_rejected_ground,
        #[cfg(test)]
        rejected_lot_ground,
        nodes,
        roads,
        lots,
        lot_ground,
        blocks: block_outlines,
        block_parents,
        gateway,
        station,
        rail_gateway,
        rail_terminus,
        founding_roads,
        founding_market,
        rail_service,
        rail_extension,
        expressway,
    }
}

impl MasterPlan {
    /// FNV-1a digest of the plan's exact bits. Saves record it so restore can
    /// prove the regenerated plan is identical on this target.
    pub fn digest(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        let mut feed = |value: u32| {
            for byte in value.to_le_bytes() {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
        };
        // Rail platform placement and access are part of the realized plan
        // contract. Changing this salt rejects saves with stale station paths.
        feed(0x5250_4c03); // Four-car platforms and independent parent/block streams.
                           // Passage-facing parcels have a different realization/validation
                           // contract even when their underlying road layout is unchanged.
        if cfg!(feature = "parent-passage-candidate") {
            feed(0x5050_0101);
        }
        if let Some(alignment) = &self.rail_alignment {
            feed(0x5241_4901);
            for (distance, point) in alignment.samples() {
                let tangent = alignment.at_x(point.x).1;
                for value in [*distance, point.x, point.z, tangent.x, tangent.z] {
                    feed(value.to_bits());
                }
            }
        }
        for node in &self.nodes {
            feed(node.p.x.to_bits());
            feed(node.p.z.to_bits());
        }
        for road in &self.roads {
            feed(road.a as u32);
            feed(road.b as u32);
            feed(road.kind as u32);
            feed(road.deck_height.to_bits());
            feed(road.curve.is_some() as u32);
            if let Some(controls) = road.curve {
                for p in controls {
                    feed(p.x.to_bits());
                    feed(p.z.to_bits());
                }
            }
        }
        for lot in &self.lots {
            for value in [
                lot.x,
                lot.z,
                lot.width,
                lot.depth,
                lot.facing.x,
                lot.facing.z,
            ] {
                feed(value.to_bits());
            }
            feed(lot.road as u32);
            if let Some(passage) = lot.passage {
                feed(0x5041_5301);
                for p in passage {
                    feed(p.x.to_bits());
                    feed(p.z.to_bits());
                }
            }
        }
        hash
    }

    /// Total lot area divided by the plan extent, as a quick density proxy.
    fn lot_coverage(&self) -> f32 {
        let area: f32 = self.lots.iter().map(|l| l.width * l.depth).sum();
        area / (2.0 * PLAN_EXTENT).powi(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn street_cut_rejects_large_area_thin_wedge_but_preserves_narrow_parent() {
        let rectangle = |w: f32, d: f32| {
            ccw(vec![
                (P::new(0.0, 0.0), None),
                (P::new(w, 0.0), None),
                (P::new(w, d), None),
                (P::new(0.0, d), None),
            ])
        };
        let parent = rectangle(100.0, 100.0);
        let normal = P::new(1.0, 1.0).unit();
        let a = clip(&parent, P::new(20.0, 20.0), normal, None);
        let b = clip(&parent, P::new(20.0, 20.0), normal.scale(-1.0), None);
        assert!(area(&a) > 500.0 && area(&b) > 500.0);
        assert!(!cut_preserves_block_depth(&parent, &a, &b, 40.0));
        let narrow = rectangle(120.0, 30.0);
        let a = clip(&narrow, P::new(60.0, 0.0), P::new(1.0, 0.0), None);
        let b = clip(&narrow, P::new(60.0, 0.0), P::new(-1.0, 0.0), None);
        assert!(cut_preserves_block_depth(&narrow, &a, &b, 40.0));
    }
    use crate::geometry;
    use std::collections::BTreeSet;

    #[test]
    fn corner_lot_uses_other_real_frontage_when_long_side_cannot_fit() {
        let mut poly = vec![
            (P::new(0.0, 0.0), Some(10)),
            (P::new(6.0, 0.0), Some(11)),
            (P::new(6.0, 22.0), None),
            (P::new(0.0, 22.0), None),
        ];
        assert!(inscribed_rect(&poly, 1, 28.0).is_none()); // Only 6 m of depth from the long side.
        assert!(fit_street_rectangle(&poly, 28.0, false).is_none());
        let (edge, line, _, width, depth, facing) =
            fit_street_rectangle(&poly, 28.0, true).unwrap();
        assert_eq!((edge, line), (0, 10));
        assert!(width >= 5.0 && depth >= 7.0);
        assert_eq!((facing.x, facing.z), (0.0, -1.0));
        poly[0].1 = None; // A cut boundary cannot supply the missing street access.
        assert!(fit_street_rectangle(&poly, 28.0, true).is_none());
    }

    #[test]
    fn viable_preferred_frontage_keeps_its_existing_rectangle() {
        let poly = vec![
            (P::new(0.0, 0.0), Some(10)),
            (P::new(12.0, 0.0), Some(11)),
            (P::new(12.0, 22.0), None),
            (P::new(0.0, 22.0), None),
        ];
        let before = inscribed_rect(&poly, 1, 28.0).unwrap();
        let (edge, line, p, w, d, facing) = fit_street_rectangle(&poly, 28.0, true).unwrap();
        assert_eq!((edge, line), (1, 11));
        assert_eq!(
            (p.x, p.z, w, d, facing.x, facing.z),
            (before.0.x, before.0.z, before.1, before.2, before.3.x, before.3.z)
        );
    }

    #[test]
    fn curved_frontage_is_finite_without_removing_land() {
        let mut b = Builder {
            nodes: vec![],
            index: BTreeMap::new(),
            roads: vec![],
            lines: vec![],
            line_lookup: BTreeMap::new(),
        };
        let a = b.node(P::new(0.0, 0.0), NodeKind::Junction);
        let end = b.node(P::new(20.0, 100.0), NodeKind::Junction);
        let line = b.line(a, end, RoadKind::Arterial);
        b.lines[line].path = vec![P::new(0.0, 0.0), P::new(0.0, 50.0), P::new(20.0, 100.0)];
        let poly = vec![
            (P::new(-20.0, -20.0), None),
            (P::new(-10.0, -20.0), Some(line)),
            (P::new(-10.0, 80.0), None),
            (P::new(-20.0, 80.0), None),
        ];
        let trimmed = trim_curve_frontage(poly.clone(), &b);
        assert!((area(&trimmed) - area(&poly)).abs() < 0.001);
        let frontage: f32 = (0..trimmed.len())
            .filter(|&i| trimmed[i].1 == Some(line))
            .map(|i| trimmed[(i + 1) % trimmed.len()].0.sub(trimmed[i].0).len())
            .sum();
        assert!((frontage - 50.0).abs() < 0.001);
    }

    #[test]
    fn concave_bank_decomposition_preserves_area_and_frontage() {
        let poly: Poly = [
            P::new(0.0, 0.0),
            P::new(80.0, 0.0),
            P::new(80.0, 100.0),
            P::new(0.0, 100.0),
            P::new(25.0, 65.0),
            P::new(10.0, 40.0),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, p)| (p, Some(i)))
        .collect();
        let pieces = convex_pieces(poly.clone());
        assert!(pieces.len() > 1 && pieces.len() < poly.len() - 2);
        let total: f32 = pieces.iter().map(area).sum();
        assert!((total - area(&poly)).abs() < 0.01);
        for piece in &pieces {
            for i in 0..piece.len() {
                let a = piece[i].0;
                let b = piece[(i + 1) % piece.len()].0;
                let c = piece[(i + 2) % piece.len()].0;
                assert!(b.sub(a).cross(c.sub(b)) >= -0.001);
            }
        }
        for i in 0..poly.len() {
            let expected = poly[(i + 1) % poly.len()].0.sub(poly[i].0).len();
            let actual: f32 = pieces
                .iter()
                .flat_map(|piece| {
                    (0..piece.len()).filter_map(move |j| {
                        (piece[j].1 == Some(i))
                            .then(|| piece[(j + 1) % piece.len()].0.sub(piece[j].0).len())
                    })
                })
                .sum();
            assert!((actual - expected).abs() < 0.001, "frontage {i}");
        }
    }

    #[test]
    fn sampled_bank_keeps_bends_and_nearby_shared_junction_connected() {
        let mut b = Builder {
            nodes: vec![],
            index: BTreeMap::new(),
            roads: vec![],
            lines: vec![],
            line_lookup: BTreeMap::new(),
        };
        let path = vec![P::new(0.0, 0.0), P::new(20.0, 50.0), P::new(0.0, 100.0)];
        let a = b.node(path[0], NodeKind::Junction);
        let end = b.node(path[2], NodeKind::Junction);
        let line = b.line(a, end, RoadKind::Arterial);
        b.lines[line].path = path.clone();
        let junction = b.split(line, path[1].lerp(path[2], 0.01));
        let side = b.node(P::new(80.0, 50.5), NodeKind::Junction);
        b.line(junction, side, RoadKind::Local);
        b.resolve_lines((200.0, 200.0, 210.0, 210.0));
        let resolved = &b.lines[line].roads;
        assert_eq!(resolved.len(), 3);
        assert_eq!(b.roads[resolved[0].2].a, a);
        assert_eq!(b.roads[resolved[2].2].b, end);
        for pair in resolved.windows(2) {
            assert_eq!(b.roads[pair[0].2].b, b.roads[pair[1].2].a);
        }
        assert_eq!(b.roads[resolved[1].2].b, junction);
        assert!((path_param(&path, path[1]) - 0.5).abs() < 0.0001);
        let entrance = path[1].lerp(path[2], 0.01);
        assert!(
            path_position(&path, path_param(&path, entrance))
                .sub(entrance)
                .len()
                < 0.001
        );
    }

    fn geography(seed: u32) -> Geography {
        Geography {
            seed,
            river: crate::river_course::sampled(seed).to_vec(),
            river_clearance: 38.0,
            protected: (-350.0, -350.0, -260.0, -270.0),
        }
    }

    fn rect(l: &PlanLot) -> geometry::OrientedRect {
        geometry::OrientedRect {
            x: l.x,
            z: l.z,
            width: l.width,
            depth: l.depth,
            rotation: l.rotation,
        }
    }

    #[test]
    fn roji_reserves_only_corridor_and_labels_real_frontage() {
        let block = vec![
            (P::new(-20.0, 0.0), Some(0)),
            (P::new(20.0, 0.0), Some(1)),
            (P::new(20.0, 60.0), Some(2)),
            (P::new(-20.0, 60.0), Some(3)),
        ];
        let pieces = reserve_roji(&block, P::new(0.0, -4.0), P::new(0.0, 40.0), 4);
        let setback = RoadKind::Lane.lot_setback();
        let expected = 40.0 * 60.0 - 2.0 * setback * (40.0 + setback);
        assert!((pieces.iter().map(area).sum::<f32>() - expected).abs() < 0.01);
        let mut frontage = 0.0;
        for piece in &pieces {
            for i in 0..piece.len() {
                let (a, label) = piece[i];
                let b = piece[(i + 1) % piece.len()].0;
                if label == Some(4) {
                    assert!(a.z <= 40.001 && b.z <= 40.001);
                    assert!((a.x.abs() - setback).abs() < 0.001);
                    assert!((b.x.abs() - setback).abs() < 0.001);
                    frontage += b.sub(a).len();
                }
                assert!(a.z >= 40.0 + setback - 0.001 || a.x.abs() >= setback - 0.001);
            }
        }
        assert!((frontage - 80.0).abs() < 0.01);
    }

    #[test]
    fn single_expressway_candidate_is_a_separate_boundary_chain() {
        for seed in [0, 5, 7, 41, 1234, 87231] {
            let geo = geography(seed);
            let plan = generate_layout(&geo, true, false);
            assert_eq!(plan.digest(), generate_layout(&geo, true, false).digest());
            let first = plan.roads[*plan.expressway.first().unwrap()];
            let last = plan.roads[*plan.expressway.last().unwrap()];
            assert_eq!(plan.nodes[first.a].p.x, EDGE);
            assert_eq!(plan.nodes[last.b].p.x, -EDGE);
            let surface: BTreeSet<_> = plan
                .roads
                .iter()
                .filter(|r| r.kind != RoadKind::Expressway)
                .flat_map(|r| [r.a, r.b])
                .collect();
            for (i, &index) in plan.expressway.iter().enumerate() {
                let road = plan.roads[index];
                assert!(!surface.contains(&road.a) && !surface.contains(&road.b));
                assert_eq!(road.deck_height, 11.0);
                if i > 0 {
                    assert_eq!(plan.roads[plan.expressway[i - 1]].b, road.a);
                }
                let a = plan.nodes[road.a].p;
                let end = plan.nodes[road.b].p;
                for step in 0..=128 {
                    let point =
                        crate::road_curve::sample(a, road.curve.unwrap(), end, step as f32 / 128.0);
                    for lot in &plan.lots {
                        assert!(
                            quad_segment_distance(&rect_corners(lot), point, point) >= 10.79,
                            "seed {seed}, road {index}: elevated candidate overlaps lot"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn plan_is_deterministic() {
        let a = generate(&geography(87231));
        let b = generate(&geography(87231));
        assert_eq!(a.lots, b.lots);
        assert_eq!(a.roads, b.roads);
        let c = generate(&geography(7));
        assert_ne!(a.lots, c.lots);
    }

    #[test]
    fn elevated_curves_clear_lots_and_share_tangents() {
        for seed in [0, 7, 41, 2026, 87231] {
            let plan = generate(&geography(seed));
            for &index in &plan.expressway {
                let road = plan.roads[index];
                let a = plan.nodes[road.a].p;
                let b = plan.nodes[road.b].p;
                let controls = road.curve.unwrap();
                // Independent dense point witness, distinct from generation's
                // conservative capsule subdivision, catches deck/parcel overlap.
                for i in 0..=128 {
                    let p = crate::road_curve::sample(a, controls, b, i as f32 / 128.0);
                    for lot in &plan.lots {
                        if (p.x - lot.x).abs() > lot.width + lot.depth + 12.0
                            || (p.z - lot.z).abs() > lot.width + lot.depth + 12.0
                        {
                            continue;
                        }
                        assert!(
                            quad_segment_distance(&rect_corners(lot), p, p) >= 10.79,
                            "seed {seed}, road {index}, lot {} {}",
                            lot.x,
                            lot.z
                        );
                    }
                }
                for &other_index in &plan.expressway {
                    let other = plan.roads[other_index];
                    if other.a == road.b && other.deck_height == road.deck_height {
                        let incoming = b.sub(controls[1]).unit();
                        let outgoing = other.curve.unwrap()[0].sub(b).unit();
                        assert!(incoming.sub(outgoing).len()<0.0001, "seed {seed} roads {index} {other_index} a {a:?} joint {b:?} end {:?} tangents {incoming:?} {outgoing:?}",plan.nodes[other.b].p);
                    }
                }
            }
            let mut changed = plan.clone();
            let index = changed.expressway[0];
            changed.roads[index].curve.as_mut().unwrap()[0].x += 0.1;
            assert_ne!(plan.digest(), changed.digest());
        }
    }

    #[test]
    fn elevated_boundary_tails_follow_their_chord_across_seed_corpus() {
        // Exercise both layouts independently of the production feature selection.
        for (seed, two_routes) in [0, 7, 41, 2026, 87231, 1, 42, 1204, 1234, 65535]
            .into_iter()
            .flat_map(|seed| [false, true].map(|two| (seed, two)))
        {
            let plan = generate_layout(&geography(seed), true, two_routes);
            assert_eq!(
                plan.expressway.len(),
                if two_routes { 16 } else { 8 },
                "seed {seed}: expressway segment count (two routes: {two_routes})"
            );
            for &index in &plan.expressway {
                let road = plan.roads[index];
                let a = plan.nodes[road.a].p;
                let b = plan.nodes[road.b].p;
                assert!(
                    a.x.is_finite() && a.z.is_finite() && b.x.is_finite() && b.z.is_finite(),
                    "seed {seed}, road {index}: non-finite endpoint"
                );
                assert!(
                    a.x.abs() <= EDGE
                        && a.z.abs() <= EDGE
                        && b.x.abs() <= EDGE
                        && b.z.abs() <= EDGE,
                    "seed {seed}, road {index}: endpoint outside boundary: {a:?} {b:?}"
                );
                let tail = [a, b]
                    .iter()
                    .any(|p| p.x.abs() == EDGE || p.z.abs() == EDGE);
                if tail {
                    let [c0, c1] = road.curve.expect("elevated road missing controls");
                    let chord = b.sub(a);
                    let length = chord.len();
                    assert!(
                        length > 1.0e-5,
                        "seed {seed}, road {index}: zero-length boundary tail"
                    );
                    // Collinearity is checked as sine of the angle, so the
                    // tolerance remains meaningful across the 10 m tails.
                    for control in [c0, c1] {
                        let offset = control.sub(a);
                        let normalized_cross = (chord.x * offset.z - chord.z * offset.x).abs()
                            / (length * offset.len().max(1.0e-5));
                        assert!(normalized_cross <= 1.0e-5,
                            "seed {seed}, road {index}: boundary curve bows from chord ({normalized_cross})");
                    }
                }
            }
        }
    }

    #[test]
    fn lots_are_disjoint_and_dense() {
        for seed in [0, 7, 41, 2026, 87231] {
            let plan = generate(&geography(seed));
            assert!(
                plan.lots.len() > 1_200,
                "seed {seed}: {} lots",
                plan.lots.len()
            );
            assert!(plan.lots.len() <= MAX_PLAN_LOTS);
            let coverage = plan.lot_coverage();
            let mut areas: Vec<f32> = plan.lots.iter().map(|l| l.width * l.depth).collect();
            areas.sort_by(f32::total_cmp);
            eprintln!(
                "seed {seed}: lots {} roads {} coverage {coverage:.3} area p50 {:.0} p90 {:.0}",
                plan.lots.len(),
                plan.roads.len(),
                areas[areas.len() / 2],
                areas[areas.len() * 9 / 10]
            );
            assert!(coverage > 0.36, "seed {seed}: lot coverage {coverage}");
            for (i, a) in plan.lots.iter().enumerate() {
                let ra = rect(a);
                for b in plan.lots.iter().skip(i + 1) {
                    if (a.x - b.x).abs() > 40.0 || (a.z - b.z).abs() > 40.0 {
                        continue;
                    }
                    // Allow shared lot lines; forbid real overlap.
                    let shrunk = geometry::OrientedRect {
                        width: ra.width - 0.1,
                        depth: ra.depth - 0.1,
                        ..ra
                    };
                    assert!(
                        !geometry::rects_overlap(&shrunk, &rect(b), 0.0),
                        "seed {seed}: lots overlap {a:?} {b:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn lots_clear_streets_and_face_their_frontage_road() {
        for seed in [87231, 0, 5] {
            let plan = generate(&geography(seed));
            for lot in &plan.lots {
                let road = plan.roads[lot.road];
                assert!(road.kind.is_street());
                let (a, b) = (plan.nodes[road.a].p, plan.nodes[road.b].p);
                let r = rect(lot);
                // The lot line sits at the road's setback, never inside it.
                let clearance = geometry::rect_segment_distance(&r, a.x, a.z, b.x, b.z);
                assert!(
                    clearance >= road.kind.lot_setback() - 0.06,
                    "lot intrudes on frontage road: {clearance} {lot:?}"
                );
                let reach = if lot.flag {
                    34.0
                } else {
                    road.kind.lot_setback() + 1.5
                };
                assert!(clearance <= reach, "seed {seed}: lot is not at its frontage: {clearance}, {lot:?}, road {a:?} -> {b:?}");
                // Local +Z faces the road.
                let facing = P::new(lot.rotation.sin(), lot.rotation.cos());
                let mid = a.lerp(b, 0.5);
                let to_road_line = {
                    let d = b.sub(a);
                    let t = P::new(lot.x, lot.z).sub(a).dot(d) / d.dot(d);
                    a.lerp(b, t).sub(P::new(lot.x, lot.z))
                };
                let _ = mid;
                assert!(
                    facing.dot(to_road_line.unit()) > 0.95,
                    "seed {seed}: lot does not face its road: {lot:?}, {a:?} -> {b:?}"
                );
                for other in &plan.roads {
                    if !other.kind.is_street() {
                        continue;
                    }
                    let (oa, ob) = (plan.nodes[other.a].p, plan.nodes[other.b].p);
                    let d = geometry::rect_segment_distance(&r, oa.x, oa.z, ob.x, ob.z);
                    assert!(
                        d >= other.kind.lot_setback() - 0.06,
                        "lot intrudes on {:?} by {d}",
                        other.kind
                    );
                }
            }
        }
    }

    #[test]
    #[cfg(feature = "flag-passage-candidate")]
    fn reserved_flag_passages_clear_all_planned_buildings() {
        for seed in [87231, 0, 5] {
            let plan = generate(&geography(seed));
            let mut count = 0;
            for lot in &plan.lots {
                if !lot.flag {
                    continue;
                }
                let passage = lot
                    .passage
                    .expect("every rear lot needs a reserved passage");
                count += 1;
                for segment in passage.windows(2) {
                    for other in &plan.lots {
                        let distance = geometry::rect_segment_distance(
                            &rect(other),
                            segment[0].x,
                            segment[0].z,
                            segment[1].x,
                            segment[1].z,
                        );
                        assert!(
                            distance >= 1.05,
                            "seed {seed}: passage {:?} hits {:?}, distance {distance}",
                            lot,
                            other
                        );
                    }
                }
            }
            assert!(count > 0, "seed {seed}: retain usable rear sites");
        }
    }

    #[test]
    fn expressway_crosses_the_river_but_never_runs_along_it() {
        for seed in [0, 7, 87231] {
            let geo = geography(seed);
            let plan = generate(&geo);
            let mut wet = 0.0;
            for &index in &plan.expressway {
                let road = plan.roads[index];
                let (a, b) = (plan.nodes[road.a].p, plan.nodes[road.b].p);
                let steps = (a.sub(b).len() / 2.0).ceil() as usize;
                for i in 0..steps {
                    let p = a.lerp(b, (i as f32 + 0.5) / steps as f32);
                    if river_distance(&geo.river, p) < geo.river_clearance {
                        wet += a.sub(b).len() / steps as f32;
                    }
                }
                // Grade separation: no elevated node is a street junction.
                assert_eq!(plan.nodes[road.a].kind, NodeKind::Elevated);
                assert_eq!(plan.nodes[road.b].kind, NodeKind::Elevated);
            }
            // One perpendicular crossing of a ~76 m river corridor.
            assert!(
                wet < 110.0,
                "seed {seed}: expressway runs {wet} m over the river"
            );
        }
    }

    #[test]
    fn expressway_corridors_cross_as_separate_decks_without_a_junction() {
        // Crossing-layer coverage needs an explicit two-route fixture, even when
        // the single-route candidate is selected for ordinary engine generation.
        let mut plan = generate_layout(&geography(87231), true, true);
        let horizontal: Vec<usize> = plan
            .expressway
            .iter()
            .copied()
            .filter(|&i| plan.roads[i].deck_height == 11.0)
            .collect();
        let vertical: Vec<usize> = plan
            .expressway
            .iter()
            .copied()
            .filter(|&i| plan.roads[i].deck_height == 20.0)
            .collect();
        let horizontal_nodes: BTreeSet<usize> = horizontal
            .iter()
            .flat_map(|&i| {
                let road = plan.roads[i];
                [road.a, road.b]
            })
            .collect();
        let vertical_nodes: BTreeSet<usize> = vertical
            .iter()
            .flat_map(|&i| {
                let road = plan.roads[i];
                [road.a, road.b]
            })
            .collect();
        assert!(
            horizontal_nodes.is_disjoint(&vertical_nodes),
            "crossing corridors share a node"
        );
        assert!(horizontal
            .iter()
            .all(|&i| plan.roads[i].deck_height == 11.0));
        assert!(vertical.iter().all(|&i| plan.roads[i].deck_height == 20.0));
        // Independent corridors cross between nodes after route fairing.
        // Check their sampled curves, rather than requiring coincident anchors.
        let segments = |indices: &[usize]| -> Vec<(P, P)> {
            indices
                .iter()
                .flat_map(|&index| {
                    let road = plan.roads[index];
                    let a = plan.nodes[road.a].p;
                    let b = plan.nodes[road.b].p;
                    (0..128).map(move |i| {
                        (
                            crate::road_curve::sample(a, road.curve.unwrap(), b, i as f32 / 128.0),
                            crate::road_curve::sample(
                                a,
                                road.curve.unwrap(),
                                b,
                                (i + 1) as f32 / 128.0,
                            ),
                        )
                    })
                })
                .collect()
        };
        let cross = |a: P, b: P| a.x * b.z - a.z * b.x;
        let horizontal_segments = segments(&horizontal);
        let vertical_segments = segments(&vertical);
        let crosses = horizontal_segments.iter().any(|&(a, b)| {
            vertical_segments.iter().any(|&(c, d)| {
                let ab = b.sub(a);
                let cd = d.sub(c);
                let denominator = cross(ab, cd);
                if denominator.abs() < 1.0e-6 {
                    return false;
                }
                let t = cross(c.sub(a), cd) / denominator;
                let u = cross(c.sub(a), ab) / denominator;
                (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)
            })
        });
        assert!(crosses, "expressway corridors should cross spatially");
        assert_eq!(
            generate_layout(&geography(87231), true, true).digest(),
            plan.digest()
        );
        let previous = plan.digest();
        plan.roads[vertical[0]].deck_height = 11.0;
        assert_ne!(
            previous,
            plan.digest(),
            "deck changes must invalidate saved plans"
        );
    }

    #[test]
    fn street_graph_is_connected_from_gateway() {
        let plan = generate(&geography(0));
        let mut adjacency: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for road in plan.roads.iter().filter(|r| r.kind.is_street()) {
            adjacency.entry(road.a).or_default().push(road.b);
            adjacency.entry(road.b).or_default().push(road.a);
        }
        let mut seen = vec![false; plan.nodes.len()];
        let mut stack = vec![plan.gateway];
        seen[plan.gateway] = true;
        while let Some(n) = stack.pop() {
            for &m in adjacency.get(&n).into_iter().flatten() {
                if !seen[m] {
                    seen[m] = true;
                    stack.push(m);
                }
            }
        }
        for lot in &plan.lots {
            let road = plan.roads[lot.road];
            assert!(seen[road.a] && seen[road.b], "lot frontage unreachable");
        }
        assert!(seen[plan.station]);
    }

    #[test]
    fn continuous_elevated_reservation_preserves_frontage_without_sample_slivers() {
        let block = vec![
            (P::new(-20.0, -20.0), Some(1)),
            (P::new(20.0, -20.0), Some(2)),
            (P::new(20.0, 20.0), Some(3)),
            (P::new(-20.0, 20.0), Some(4)),
        ];
        let segments: Vec<_> = (-50..50)
            .map(|x| (P::new(x as f32, 0.0), P::new((x + 1) as f32, 0.0), 5.0))
            .collect();
        let pieces = reserve_elevated_corridor(block, &segments);
        assert_eq!(
            pieces.len(),
            2,
            "one piece per side, independent of sample count"
        );
        assert!((pieces.iter().map(area).sum::<f32>() - 1200.0).abs() < 0.01);
        for piece in pieces {
            assert!(
                piece.iter().any(|p| p.1.is_some()),
                "original street access survives"
            );
            for i in 0..piece.len() {
                let (a, label) = piece[i];
                let b = piece[(i + 1) % piece.len()].0;
                assert!(a.z.abs() >= 4.999);
                if a.z.abs() < 5.001 && b.z.abs() < 5.001 {
                    assert_eq!(label, None);
                }
            }
        }
    }

    #[test]
    fn alternate_frontage_preserves_existing_lots_in_each_corridor_layout() {
        for seed in [87231, 0, 5] {
            for curved in [false, true] {
                for crossing in [false, true] {
                    let geo = geography(seed);
                    let before = generate_layout_with_frontage(&geo, curved, crossing, false);
                    let after = generate_layout_with_frontage(&geo, curved, crossing, true);
                    assert_eq!(before.nodes, after.nodes);
                    assert_eq!(before.roads, after.roads);
                    assert_eq!(before.blocks, after.blocks);
                    for (index, lot) in before.lots.iter().enumerate() {
                        let kept=after.lots.iter().position(|candidate| candidate==lot)
                            .expect("alternate frontage must preserve every previously admitted lot and access contract");
                        assert_eq!(before.lot_ground[index], after.lot_ground[kept]);
                    }
                    assert!(
                        after.lots.len() > before.lots.len(),
                        "seed {seed}: expected corner recovery"
                    );
                }
            }
        }
    }

    #[test]
    fn curved_rail_reserves_blocks_before_lot_admission() {
        for seed in [87231, 0, 5] {
            let geo = geography(seed);
            let control = generate_impl(&geo, false);
            let plan = generate_impl(&geo, true);
            let path: Vec<P> = plan
                .rail_alignment
                .as_ref()
                .unwrap()
                .samples()
                .iter()
                .map(|s| s.1)
                .collect();
            // Keep the original corridor intervention's 90% guard on its fixed
            // fitting policy. Alternate frontage independently preserves every
            // old lot in BOTH layouts (tested above); unequal additional corner
            // recovery must not masquerade as a loss caused by curved rail.
            let baseline_control = generate_layout_with_frontage(&geo, false, true, false);
            let baseline_plan = generate_layout_with_frontage(&geo, true, true, false);
            assert!(
                baseline_plan.lots.len() > baseline_control.lots.len() * 9 / 10,
                "seed {seed}: excessive lot loss {} -> {}",
                baseline_control.lots.len(),
                baseline_plan.lots.len()
            );
            assert_eq!(plan.rejected_lot_ground.iter().filter(|(reason, _)| *reason == "rail_clearance").count(), 0,
                "seed {seed}: reservation failed; final admission should not carve away rail-overlapping lots");
            let mut adjacency = vec![Vec::new(); plan.nodes.len()];
            for road in plan.roads.iter().filter(|r| r.kind.is_street()) {
                adjacency[road.a].push(road.b);
                adjacency[road.b].push(road.a);
                if road.kind != RoadKind::Bridge {
                    let half_width = match road.kind {
                        RoadKind::Arterial => 8.0,
                        RoadKind::Lane => 2.0,
                        _ => 3.25,
                    };
                    let a = plan.nodes[road.a].p;
                    let b = plan.nodes[road.b].p;
                    assert!(
                        geo.river
                            .windows(2)
                            .all(|w| crate::geometry::segments_distance_coords(
                                a.x, a.z, b.x, b.z, w[0].x, w[0].z, w[1].x, w[1].z
                            ) >= geo.river_clearance + half_width - 0.01),
                        "seed {seed}: street intrudes into river bank"
                    );
                }
            }
            let mut seen = vec![false; plan.nodes.len()];
            let mut stack = vec![plan.gateway];
            seen[plan.gateway] = true;
            while let Some(node) = stack.pop() {
                for &next in &adjacency[node] {
                    if !seen[next] {
                        seen[next] = true;
                        stack.push(next);
                    }
                }
            }
            for (index, lot) in plan.lots.iter().enumerate() {
                let corners = rect_corners(lot);
                let road = plan.roads[lot.road];
                assert!(
                    seen[road.a] && seen[road.b],
                    "seed {seed}, lot {index}: disconnected frontage"
                );
                assert!(
                    path.windows(2)
                        .all(|p| quad_segment_distance(&corners, p[0], p[1])
                            >= RAIL_BAND_HALF - 0.06),
                    "seed {seed}, lot {index}: rail corridor overlap"
                );
                let rect = rect(lot);
                let shrunk = geometry::OrientedRect {
                    width: rect.width - 0.1,
                    depth: rect.depth - 0.1,
                    ..rect
                };
                for other in plan.lots.iter().skip(index + 1) {
                    let reach = (lot.width + lot.depth + other.width + other.depth) * 0.5;
                    if (lot.x - other.x).abs() > reach || (lot.z - other.z).abs() > reach {
                        continue;
                    }
                    assert!(
                        !geometry::rects_overlap(
                            &shrunk,
                            &geometry::OrientedRect {
                                x: other.x,
                                z: other.z,
                                width: other.width,
                                depth: other.depth,
                                rotation: other.rotation,
                            },
                            0.0
                        ),
                        "seed {seed}, lot {index}: replanned lots overlap"
                    );
                }
            }
            assert_eq!(plan.nodes[plan.station].p, control.nodes[control.station].p);
            assert_eq!(plan.digest(), generate_impl(&geo, true).digest());
        }
    }
}

#[cfg(test)]
mod svg {
    use super::*;

    /// `PLAN_SVG=/path/plan.svg cargo test --release masterplan::svg -- --ignored`
    #[test]
    #[ignore]
    fn write_plan_svg() {
        let path = std::env::var("PLAN_SVG").unwrap_or_else(|_| "plan.svg".into());
        let seed: u32 = std::env::var("PLAN_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(87231);
        let bend = (seed % 5) as f32 * 9.0 - 18.0;
        let geo = Geography {
            seed,
            river: [
                P::new(-112.0 + bend, -400.0),
                P::new(-98.0 + bend, -190.0),
                P::new(-112.0 + bend, 20.0),
                P::new(-94.0 + bend, 210.0),
                P::new(-109.0 + bend, 400.0),
            ]
            .to_vec(),
            river_clearance: 38.0,
            protected: (-350.0, -350.0, -260.0, -270.0),
        };
        let plan = generate(&geo);
        let mut s = String::from(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-400 -400 800 800" width="1000" height="1000"><rect x="-400" y="-400" width="800" height="800" fill="#e4e8dc"/>"##,
        );
        let r = geo.river;
        s += &format!(
            r##"<polyline points="{}" fill="none" stroke="#86b2bb" stroke-width="64"/>"##,
            r.iter()
                .map(|p| format!("{},{}", p.x, -p.z))
                .collect::<Vec<_>>()
                .join(" ")
        );
        for road in &plan.roads {
            let (a, b) = (plan.nodes[road.a].p, plan.nodes[road.b].p);
            let (w, c) = match road.kind {
                RoadKind::Arterial => (14.0, "#9a9a92"),
                RoadKind::Local => (6.5, "#b3b3aa"),
                RoadKind::Bridge => (9.0, "#b3b3aa"),
                RoadKind::Lane => (4.0, "#c4c4bb"),
                RoadKind::Rail => (6.0, "#6a5d50"),
                RoadKind::Expressway => (12.0, "#5d6f78"),
            };
            s += &format!(
                r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{c}" stroke-width="{w}" stroke-opacity="0.85"/>"##,
                a.x, -a.z, b.x, -b.z
            );
        }
        for lot in &plan.lots {
            let fill = match lot.district {
                District::StationCore => "#2f4b55",
                District::StationFringe => "#4f6b73",
                District::Compact => "#71858a",
                District::Strips => "#7d8a7a",
                District::Waterfront => "#9aa6a0",
            };
            let c = rect_corners(lot);
            s += &format!(
                r##"<polygon points="{}" fill="{fill}" stroke="#e4e8dc" stroke-width="0.25"/>"##,
                c.iter()
                    .map(|p| format!("{:.2},{:.2}", p.x, -p.z))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
        let st = plan.nodes[plan.station].p;
        s += &format!(
            r##"<circle cx="{}" cy="{}" r="6" fill="#c0392b"/></svg>"##,
            st.x, -st.z
        );
        std::fs::write(path, s).unwrap();
    }
}

#[cfg(test)]
mod export {
    use super::*;

    /// `PLAN_JSON=/path PLAN_SEED=87231 cargo test --release masterplan::export -- --ignored`
    /// Exports the plan for scripts/assets/plateau-street-reference.py.
    #[test]
    #[ignore]
    fn export_plan_json() {
        let path = std::env::var("PLAN_JSON").unwrap_or_else(|_| "plan.json".into());
        let seed: u32 = std::env::var("PLAN_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(87231);
        let river = crate::river_course::sampled(seed).to_vec();
        let plan = generate_layout(
            &Geography {
                seed,
                river: river.clone(),
                river_clearance: 38.0,
                protected: (-350.0, -350.0, -260.0, -270.0),
            },
            std::env::var("PLAN_CURVED_RAIL").as_deref() != Ok("0"),
            std::env::var("PLAN_SINGLE_EXPRESSWAY").as_deref() != Ok("1"),
        );
        let nodes: Vec<[f32; 2]> = plan.nodes.iter().map(|n| [n.p.x, n.p.z]).collect();
        let roads: Vec<serde_json::Value> = plan
            .roads
            .iter()
            .map(|r| serde_json::json!({"a": r.a, "b": r.b, "kind": r.kind.as_str()}))
            .collect();
        let lots: Vec<[f32; 6]> = plan
            .lots
            .iter()
            .map(|l| [l.x, l.z, l.width, l.depth, l.facing.x, l.facing.z])
            .collect();
        let river_street_clearance: Vec<_> = plan
            .roads
            .iter()
            .enumerate()
            .filter_map(|(index, road)| {
                let half_width = match road.kind {
                    RoadKind::Arterial => 8.0,
                    RoadKind::Local => 3.25,
                    RoadKind::Lane => 2.0,
                    _ => return None,
                };
                let (a, b) = (plan.nodes[road.a].p, plan.nodes[road.b].p);
                let distance = river
                    .windows(2)
                    .map(|w| {
                        crate::geometry::segments_distance_coords(
                            a.x, a.z, b.x, b.z, w[0].x, w[0].z, w[1].x, w[1].z,
                        )
                    })
                    .fold(f32::INFINITY, f32::min);
                Some(serde_json::json!({"road":index,"kind":road.kind.as_str(),
                "distance":distance,"required":38.0+half_width,"margin":distance-38.0-half_width}))
            })
            .collect();
        let json = serde_json::json!({
            "railAlignment": plan.rail_alignment.as_ref().map(|path| path.samples().iter().map(|(_, p)| [p.x,p.z]).collect::<Vec<_>>()),
            "planDigest": plan.digest().to_string(),
            "blockLandBudget": plan.block_land_budget,
            "blockParents": plan.block_parents,
            "streetLandBudget": plan.street_land_budget,
            "streetSubdivisions": plan.street_subdivisions,
            "partitionRejectedGround": plan.partition_rejected_ground.iter().map(|(reason, poly)| serde_json::json!({
                "reason": reason, "points": poly.iter().map(|p| [p.x,p.z]).collect::<Vec<_>>()
            })).collect::<Vec<_>>(),
            "riverStreetClearance": river_street_clearance,
            "seed": seed,
            "extent": 400.0,
            "river": river.iter().map(|p| [p.x, p.z]).collect::<Vec<_>>(),
            "riverHalfWidth": 32.0,
            "nodes": nodes,
            "roads": roads,
            "lots": lots,
            "lotAccess": plan.lots.iter().map(|lot| serde_json::json!({
                "flag":lot.flag,"road":lot.road,"block":lot.block,
                "passage":lot.passage.map(|ps|ps.map(|p|[p.x,p.z]))
            })).collect::<Vec<_>>(),
            "lotGround": plan.lot_ground.iter().map(|poly| poly.iter().map(|p| [p.x,p.z]).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "blocks": plan.blocks.iter().map(|poly| poly.iter().map(|p| [p.x,p.z]).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "rejectedLotGround": plan.rejected_lot_ground.iter().map(|(reason, poly)| serde_json::json!({
                "reason": reason, "points": poly.iter().map(|p| [p.x,p.z]).collect::<Vec<_>>()
            })).collect::<Vec<_>>(),
        });
        std::fs::write(path, json.to_string()).unwrap();
    }
}

#[cfg(all(test, feature = "frontage-network-candidate"))]
mod frontage_network_tests {
    use super::*;
    #[test]
    fn fringe_keeps_one_connector_and_rear_sites_have_reserved_passages() {
        for seed in [87231, 0, 5] {
            let geo = Geography {
                seed,
                river: crate::river_course::sampled(seed).to_vec(),
                river_clearance: 38.0,
                protected: (-350.0, -350.0, -260.0, -270.0),
            };
            let plan = generate(&geo);
            let fringe: Vec<_> = plan
                .street_subdivisions
                .iter()
                .filter(|c| c.district == "StationFringe")
                .collect();
            assert!(!fringe.is_empty());
            assert!(fringe.iter().all(|c| c.depth == 0));
            assert_eq!(
                fringe.len(),
                fringe
                    .iter()
                    .map(|c| c.coarse_cell)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
            );
            assert!(plan
                .lots
                .iter()
                .filter(|l| l.flag)
                .all(|l| l.passage.is_some()));
            assert!(plan
                .lots
                .iter()
                .any(|l| l.district == District::StationFringe && l.passage.is_some()));
            assert_eq!(plan.digest(), generate(&geo).digest());
            let straight = generate_impl(&geo, false);
            let mut stable_parents = 0;
            for root in plan.street_subdivisions.iter().filter(|c| c.depth == 0) {
                if let Some(other) = straight.street_subdivisions.iter().find(|c| {
                    c.depth == 0 && c.coarse_cell == root.coarse_cell && c.parent == root.parent
                }) {
                    assert_eq!(
                        root.district, other.district,
                        "rail geometry must not reroll an unchanged coarse parent"
                    );
                    stable_parents += 1;
                }
            }
            assert!(stable_parents >= 10, "exercise distant unchanged parents");
        }
    }
}

#[cfg(test)]
mod local_stream_tests {
    use super::*;
    #[test]
    fn unchanged_parent_inputs_preserve_subdivision_across_rail_geometry_changes() {
        for seed in [87231, 0, 5] {
            let geo = Geography {
                seed,
                river: crate::river_course::sampled(seed).to_vec(),
                river_clearance: 38.0,
                protected: (-350.0, -350.0, -260.0, -270.0),
            };
            let straight = generate_impl(&geo, false);
            let curved = generate_impl(&geo, true);
            let mut checked = 0;
            for root in curved.street_subdivisions.iter().filter(|c| c.depth == 0) {
                if straight.street_subdivisions.iter().any(|c| {
                    c.depth == 0
                        && c.coarse_cell == root.coarse_cell
                        && c.parent == root.parent
                        && c.input_junctions == root.input_junctions
                        && c.district == root.district
                }) {
                    let signature = |plan: &MasterPlan| {
                        plan.street_subdivisions
                            .iter()
                            .filter(|c| c.coarse_cell == root.coarse_cell)
                            .map(|c| {
                                (
                                    c.depth,
                                    c.district.clone(),
                                    c.parent.clone(),
                                    c.endpoints,
                                    c.kind.clone(),
                                )
                            })
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(
                        signature(&straight),
                        signature(&curved),
                        "seed {seed}, unchanged parent {:?}",
                        root.coarse_cell
                    );
                    checked += 1;
                }
            }
            assert!(checked>=6,"exercise distant parents with identical geometry, boundary junctions and district inputs; seed {seed}, checked {checked}");
        }
    }
}
