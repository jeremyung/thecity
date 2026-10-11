//! Seeded master plan: transport skeleton, arterial lattice, ruled local
//! streets, block quads and a gap-free lot partition.
//!
//! The plan is pure data derived from the seed and river geometry. It never
//! touches engine state or simulation RNG streams, so restore can regenerate it
//! exactly. Lattice edges are straight segments between shared nodes and every
//! block is a convex quadrilateral, which keeps the street graph planar by
//! construction and lets lots tile each block without pairwise clearance gaps.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct P {
    pub(crate) x: f32,
    pub(crate) z: f32,
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
pub(crate) enum NodeKind {
    Junction,
    Gateway,
    Station,
    RailGateway,
    RailTerminus,
    Elevated,
}

impl NodeKind {
    pub(crate) fn as_str(self) -> &'static str {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RoadKind {
    Arterial,
    Local,
    Lane,
    Bridge,
    Rail,
    Expressway,
}

impl RoadKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            RoadKind::Arterial => "arterial",
            RoadKind::Local => "local",
            RoadKind::Lane => "lane",
            RoadKind::Bridge => "bridge",
            RoadKind::Rail => "rail",
            RoadKind::Expressway => "expressway",
        }
    }
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
    pub(crate) fn is_street(self) -> bool {
        matches!(
            self,
            RoadKind::Arterial | RoadKind::Local | RoadKind::Lane | RoadKind::Bridge
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum District {
    StationCore,
    StationFringe,
    Compact,
    Strips,
    Waterfront,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PlanNode {
    pub(crate) p: P,
    pub(crate) kind: NodeKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PlanRoad {
    pub(crate) a: usize,
    pub(crate) b: usize,
    pub(crate) kind: RoadKind,
    pub(crate) deck_height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PlanLot {
    pub(crate) x: f32,
    pub(crate) z: f32,
    pub(crate) width: f32,
    pub(crate) depth: f32,
    /// Three.js Y rotation; local +Z faces the frontage road.
    pub(crate) rotation: f32,
    /// Unit vector the lot faces (toward its frontage road). Plan decisions
    /// use this instead of trigonometry so they are target-independent.
    pub(crate) facing: P,
    pub(crate) road: usize,
    pub(crate) district: District,
    /// True when the frontage road is an arterial.
    pub(crate) arterial_frontage: bool,
    /// Rear flag lot reached by a passage beside a front lot.
    pub(crate) flag: bool,
    pub(crate) block: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct MasterPlan {
    pub(crate) nodes: Vec<PlanNode>,
    pub(crate) roads: Vec<PlanRoad>,
    pub(crate) lots: Vec<PlanLot>,
    /// Buildable (inset) block outline per block id; paved once developed.
    pub(crate) blocks: Vec<Vec<P>>,
    pub(crate) gateway: usize,
    pub(crate) station: usize,
    pub(crate) rail_gateway: usize,
    pub(crate) rail_terminus: usize,
    /// Plan road indices opened at founding, ordered from the gateway.
    pub(crate) founding_roads: Vec<usize>,
    /// East-bank junction where the founding market street meets the river.
    pub(crate) founding_market: usize,
    /// Service rail (station → rail gateway) and the western continuation.
    pub(crate) rail_service: usize,
    pub(crate) rail_extension: usize,
    pub(crate) expressway: Vec<usize>,
}

pub(crate) struct Geography {
    pub(crate) seed: u32,
    /// River centerline control points, ascending in z.
    pub(crate) river: [P; 5],
    pub(crate) river_clearance: f32,
    /// Axis-aligned protected rectangle (min_x, min_z, max_x, max_z).
    pub(crate) protected: (f32, f32, f32, f32),
}

pub(crate) const PLAN_EXTENT: f32 = 382.0;
pub(crate) const EDGE: f32 = 392.0;
pub(crate) const MAX_PLAN_LOTS: usize = 4_000;
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

/// A straight street line. Streets that meet it from either side split it, so
/// it resolves into several road segments sharing junction nodes.
struct Line {
    a: usize,
    b: usize,
    kind: RoadKind,
    splits: Vec<(f32, usize)>,
    /// Resolved road indices in order from `a` to `b`, with their end
    /// parameters.
    roads: Vec<(f32, f32, usize)>,
}

/// Convex polygon whose edge `i` (vertex `i` → `i + 1`) carries a label: the
/// street line it lies on, or `None` for an internal lot boundary.
type Poly = Vec<(P, Option<usize>)>;

struct PendingLot {
    lot: PlanLot,
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
        let deck_height = if kind == RoadKind::Expressway { 11.0 } else { 0.0 };
        self.roads.push(PlanRoad { a, b, kind, deck_height });
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
        });
        let id = self.lines.len() - 1;
        self.line_lookup.insert(key, id);
        id
    }
    fn line_param(&self, line: usize, p: P) -> f32 {
        let (a, b) = (self.nodes[self.lines[line].a].p, self.nodes[self.lines[line].b].p);
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
            splits.sort_by(|x, y| x.0.total_cmp(&y.0));
            points.extend(splits);
            points.push((1.0, self.lines[e].b));
            points.dedup_by(|x, y| x.1 == y.1);
            let kind = self.lines[e].kind;
            let mut resolved = Vec::new();
            for pair in points.windows(2) {
                let (pa, pb) = (self.nodes[pair[0].1].p, self.nodes[pair[1].1].p);
                if pa.sub(pb).len() < 1.0 || segment_hits_rect(pa, pb, protected, 0.0) {
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

/// Distance from a segment to a convex quad, using only IEEE-exact operations
/// so plan decisions match on every target.
fn quad_segment_distance(q: &[P; 4], a: P, b: P) -> f32 {
    let inside = |p: P| {
        let signs: Vec<f32> = (0..4).map(|i| q[(i + 1) % 4].sub(q[i]).cross(p.sub(q[i]))).collect();
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

pub(crate) fn river_x(river: &[P; 5], z: f32) -> f32 {
    for w in river.windows(2) {
        if z >= w[0].z && z <= w[1].z {
            let t = ((z - w[0].z) / (w[1].z - w[0].z)).clamp(0.0, 1.0);
            return w[0].x + (w[1].x - w[0].x) * t;
        }
    }
    if z < river[0].z {
        river[0].x
    } else {
        river[4].x
    }
}

fn river_distance(river: &[P; 5], p: P) -> f32 {
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
    (0..n).map(|i| poly[i].0.cross(poly[(i + 1) % n].0)).sum::<f32>() * 0.5
}

fn centroid(poly: &Poly) -> P {
    let n = poly.len() as f32;
    poly.iter().fold(P::new(0.0, 0.0), |acc, v| acc.add(v.0)).scale(1.0 / n)
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
        if cleaned.last().is_some_and(|last: &(P, Option<usize>)| last.0.sub(v.0).len() < 0.05) {
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
        let (mut u0, mut u1, mut v0, mut v1) = (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
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
    let deepest = poly.iter().map(|p| p.0.sub(a).dot(v)).fold(0.0, f32::max).min(max_depth);
    let front = slice(0.02)?;
    let mut best: Option<(f32, f32, f32, f32)> = None;
    for step in 1..=24 {
        let d = deepest * step as f32 / 24.0;
        let Some(back) = slice((d - 0.02).max(0.02)) else { continue };
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
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), x| (lo.min(x), hi.max(x)))
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
) {
    if area(block) < 60.0 {
        return;
    }
    // Rows follow the block's longest street edge, not its bounding box, so
    // lots line up with the street they face.
    let Some((u, _)) = longest_street_edge(block) else {
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
            continue;
        }
        let n_row = row.len();
        let Some((row_edge, row_line)) = (0..n_row)
            .filter_map(|i| row[i].1.map(|line| (i, line, row[(i + 1) % n_row].0.sub(row[i].0).len())))
            .max_by(|x, y| x.2.total_cmp(&y.2))
            .filter(|x| x.2 >= 6.0)
            .map(|(i, line, _)| (i, line))
        else {
            continue;
        };
        let row_a = row[row_edge].0;
        let ru = row[(row_edge + 1) % n_row].0.sub(row_a).unit();
        let mut rv = P::new(-ru.z, ru.x);
        if centroid(&row).sub(row_a).dot(rv) < 0.0 {
            rv = rv.scale(-1.0);
        }
        let row_depth = row.iter().map(|p| p.0.sub(row_a).dot(rv)).fold(0.0, f32::max);
        // Frontage width depends on the street the row faces.
        let arterial = is_arterial(row_line);
        let (min_w, max_w) = frontage_range(district, arterial);
        // Deep rows get a rear strip of flag lots (旗竿地): narrow rear sites
        // reached by a passage beside the front lot, addressed to the same
        // street. This keeps Tokyo's fine grain without new streets.
        let mut front_strip = row.clone();
        if !arterial && row_depth >= front_depth + 9.0 {
            let depth = front_depth + rng.range(-1.5, 1.5);
            let cut = row_a.add(rv.scale(depth));
            front_strip = clip(&row, cut, rv.scale(-1.0), None);
            let rear = clip(&row, cut, rv, None);
            if rear.len() >= 3 {
                for lot in cut_strip(rear, ru, min_w + 1.0, max_w + 2.0, rng) {
                    if area(&lot) < 40.0 {
                        continue;
                    }
                    let Some((center, width, lot_depth, outward)) =
                        inscribed_rect_frame(&lot, cut, ru, rv, max_depth)
                    else {
                        continue;
                    };
                    let front_mid = row_a.add(ru.scale(center.sub(row_a).dot(ru)));
                    out.push(PendingLot {
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
                continue;
            }
            let n = lot.len();
            let Some((edge, line)) = (0..n)
                .filter_map(|i| lot[i].1.map(|line| (i, line, lot[(i + 1) % n].0.sub(lot[i].0).len())))
                .filter(|(_, _, len)| *len >= 4.0)
                .max_by(|x, y| x.2.total_cmp(&y.2))
                .map(|(i, line, _)| (i, line))
            else {
                continue;
            };
            let Some((center, width, depth, outward)) = inscribed_rect(&lot, edge, max_depth) else {
                continue;
            };
            let a = lot[edge].0;
            let along = lot[(edge + 1) % n].0.sub(a).unit();
            let front_mid = a.add(along.scale(center.sub(a).dot(along)));
            out.push(PendingLot {
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
                    block: block_id,
                },
                front_mid,
                line,
            });
        }
    }
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
    let d = lb.sub(la);
    let t = (foot.sub(la).dot(d) / d.dot(d)).clamp(0.05, 0.95);
    let street_point = la.lerp(lb, t);
    if b.nearest_junction(line, street_point) < 12.0 {
        return None;
    }
    let start = b.split(line, street_point);
    let end = b.node(foot.add(v.scale(exit * rng.range(0.55, 0.7))), NodeKind::Junction);
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
) {
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
    let min_piece = if lv > 2.0 * min_short { min_short } else { 0.4 * max_long };
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
            continue;
        }
        // Keep new junctions away from existing ones and from corners.
        let clear = hits.iter().all(|(p, label)| {
            label.is_some_and(|line| b.nearest_junction(line, *p) > 14.0)
                && poly.iter().all(|v| v.0.sub(*p).len() > 14.0)
        });
        if !clear {
            continue;
        }
        let a_piece = clip(&poly, origin, n, None);
        let b_piece = clip(&poly, origin, n.scale(-1.0), None);
        if a_piece.len() < 3 || b_piece.len() < 3 || area(&a_piece).abs() < 500.0 || area(&b_piece).abs() < 500.0 {
            continue;
        }
        let na = b.split(hits[0].1.unwrap(), hits[0].0);
        let nb = b.split(hits[1].1.unwrap(), hits[1].0);
        let kind = if district == District::Strips && split_across_long && rng.unit() < 0.6 {
            RoadKind::Lane
        } else {
            RoadKind::Local
        };
        let line = b.line(na, nb, kind);
        let relabel = |piece: Poly| -> Poly {
            piece
                .into_iter()
                .map(|(p, label)| (p, label.or(Some(line))))
                .collect()
        };
        split_streets(b, relabel(a_piece), district, rng, depth + 1, frame, blocks);
        split_streets(b, relabel(b_piece), district, rng, depth + 1, frame, blocks);
        return;
    }
    blocks.push(poly);
}

struct LatticeLine {
    kind: LineKind,
}

pub(crate) fn generate(geo: &Geography) -> MasterPlan {
    let mut rng = Rng::new(geo.seed);
    let river = &geo.river;
    let l = PLAN_EXTENT;

    // Rows: outer ring, every river bend (so river-following columns stay
    // straight between rows), and the rail corridor's two frontage streets.
    let mut rows: Vec<(f32, LatticeLine)> = vec![(-l, LatticeLine { kind: LineKind::Ring })];
    for p in &river[1..4] {
        rows.push((p.z, LatticeLine { kind: LineKind::Collector }));
    }
    rows.push((l, LatticeLine { kind: LineKind::Ring }));
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (band_lo, band_hi) = rows
        .windows(2)
        .map(|w| (w[0].0, w[1].0))
        .filter(|(lo, hi)| *lo > -l + 1.0 && *hi < l - 1.0)
        .max_by(|x, y| ((x.1 - x.0) - x.0.abs() * 0.1).total_cmp(&((y.1 - y.0) - y.0.abs() * 0.1)))
        .unwrap_or((20.0, 210.0));
    let rail_z = (band_lo + band_hi) * 0.5 + rng.range(-12.0, 12.0);
    rows.push((rail_z - RAIL_BAND_HALF, LatticeLine { kind: LineKind::RailFrontage }));
    rows.push((rail_z + RAIL_BAND_HALF, LatticeLine { kind: LineKind::RailFrontage }));
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
    promote(&mut rows, &|z| if z > rail_z { z - rail_z } else { f32::INFINITY });

    let west_mid = 0.5 + rng.range(-0.08, 0.08);
    let east_a = 1.0 / 3.0 + rng.range(-0.05, 0.05);
    let east_b = 2.0 / 3.0 + rng.range(-0.05, 0.05);
    let bank_w = |z: f32| river_x(river, z) - BANK_OFFSET;
    let bank_e = |z: f32| river_x(river, z) + BANK_OFFSET;
    let columns: Vec<(Box<dyn Fn(f32) -> f32>, LineKind)> = vec![
        (Box::new(move |_| -l), LineKind::Ring),
        (Box::new(move |z| -l + (bank_w(z) + l) * west_mid), LineKind::Collector),
        (Box::new(bank_w), LineKind::Arterial),
        (Box::new(bank_e), LineKind::Arterial),
        (Box::new(move |z| bank_e(z) + (l - bank_e(z)) * east_a), LineKind::Arterial),
        (Box::new(move |z| bank_e(z) + (l - bank_e(z)) * east_b), LineKind::Collector),
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
                let anchor = if c < RIVER_COL { base[r][RIVER_COL].x } else { base[r][RIVER_COL + 1].x };
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
    let station_p = P::new(station_x, rail_z);
    let station_dist = |p: P| p.sub(station_p).len();

    let mut pending: Vec<PendingLot> = Vec::new();
    let mut block_outlines: Vec<Vec<P>> = Vec::new();
    let mut block_id = 0u32;
    let mut all_blocks: Vec<(Poly, District)> = Vec::new();
    for r in 0..rows.len() - 1 {
        if r == rail_band_row {
            continue;
        }
        for c in 0..columns.len() - 1 {
            if c == RIVER_COL {
                continue;
            }
            let corners = [grid[r][c], grid[r][c + 1], grid[r + 1][c + 1], grid[r + 1][c]];
            let quad: [P; 4] = corners.map(|n| b.nodes[n].p);
            // Protected land keeps only its own footprint open: streets and
            // lots touching it are filtered after subdivision.
            let poly: Poly = ccw(vec![
                (quad[0], row_lines[r][c]),
                (quad[1], Some(col_lines[r][c + 1])),
                (quad[2], row_lines[r + 1][c]),
                (quad[3], Some(col_lines[r][c])),
            ]);
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
            let frame = axis.add(P::new(-axis.z, axis.x).scale(rng.range(-turn, turn))).unit();
            split_streets(&mut b, poly, district, &mut rng, 0, frame, &mut blocks);
            for block in blocks {
                all_blocks.push((block, district));
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
        la.lerp(lb, t)
    };
    let access_node = b.split(access_line, access_point);

    // Lots, after every street exists so setbacks see final kinds.
    for (block, district) in &all_blocks {
        let line_kind = |label: Option<usize>| label.map(|l| b.lines[l].kind);
        let inner = inset(block, |label| line_kind(label).map_or(0.0, |k| k.lot_setback()));
        if inner.len() < 3 {
            continue;
        }
        let is_arterial = |line: usize| b.lines[line].kind == RoadKind::Arterial;
        let first_lot = pending.len();
        partition_block(&inner, *district, block_id, &is_arterial, &mut rng, &mut pending);
        // Dead-end roji: a 4 m alley from a local street into a deep block.
        let roji_chance = match district {
            District::Compact => 0.55,
            District::Strips => 0.45,
            District::StationFringe => 0.3,
            District::Waterfront => 0.2,
            District::StationCore => 0.1,
        };
        if rng.unit() < roji_chance {
            if let Some((start, end)) = roji(&mut b, &inner, &mut rng) {
                let stub_a = b.nodes[start].p;
                let stub_b = b.nodes[end].p;
                b.line(start, end, RoadKind::Lane);
                let mut index = first_lot;
                while index < pending.len() {
                    let corners = rect_corners(&pending[index].lot);
                    if quad_segment_distance(&corners, stub_a, stub_b) < RoadKind::Lane.lot_setback() {
                        pending.remove(index);
                    } else {
                        index += 1;
                    }
                }
            }
        }
        block_outlines.push(inner.iter().map(|v| v.0).collect());
        block_id += 1;
    }

    b.resolve_lines(geo.protected);
    b.road(access_node, station, RoadKind::Local);

    let gateway_row = rows
        .iter()
        .enumerate()
        .filter(|(_, (_, line))| line.kind == LineKind::Arterial)
        .min_by(|x, y| (x.1 .0 - rail_z + 60.0).abs().total_cmp(&(y.1 .0 - rail_z + 60.0).abs()))
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
            adjacency.entry(road.a).or_default().push((road.b, index, length));
            adjacency.entry(road.b).or_default().push((road.a, index, length));
        }
        let mut dist: BTreeMap<usize, f32> = BTreeMap::new();
        let mut prev: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
        let mut open: Vec<(f32, usize)> = vec![(0.0, founding_market)];
        dist.insert(founding_market, 0.0);
        while let Some(position) = (0..open.len()).min_by(|x, y| open[*x].0.total_cmp(&open[*y].0)) {
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

    let rail_gateway = b.node(P::new(EDGE, rail_z), NodeKind::RailGateway);
    let rail_terminus = b.node(P::new(-EDGE, rail_z), NodeKind::RailTerminus);
    let rail_service = b.road(station, rail_gateway, RoadKind::Rail);
    let rail_extension = b.road(station, rail_terminus, RoadKind::Rail);

    // Elevated expressway on land, Shuto-style: it rides above one arterial
    // row and one arterial column and crosses the river only on the row's
    // bridge, never running along the water. Elevated nodes are separate from
    // street junctions, so the expressway is grade-separated from the grid.
    let mut expressway = Vec::new();
    let mut row_elevated: BTreeMap<usize, usize> = BTreeMap::new();
    let mut row_lift = |b: &mut Builder, street_node: usize| -> usize {
        *row_elevated.entry(street_node).or_insert_with(|| {
            b.nodes.push(PlanNode { p: b.nodes[street_node].p, kind: NodeKind::Elevated });
            b.nodes.len() - 1
        })
    };
    let mut edge_node = |b: &mut Builder, p: P| -> usize {
        b.nodes.push(PlanNode { p, kind: NodeKind::Elevated });
        b.nodes.len() - 1
    };
    let expressway_row = rows
        .iter()
        .position(|(z, line)| line.kind == LineKind::Arterial && *z > rail_z)
        .or_else(|| rows.iter().position(|(_, line)| line.kind == LineKind::Arterial))
        .unwrap();
    let east_z = b.nodes[grid[expressway_row][columns.len() - 1]].p.z;
    let mut route: Vec<usize> = vec![edge_node(&mut b, P::new(EDGE, east_z))];
    for c in (0..columns.len()).rev() {
        route.push(row_lift(&mut b, grid[expressway_row][c]));
    }
    let west_z = b.nodes[grid[expressway_row][0]].p.z;
    route.push(edge_node(&mut b, P::new(-EDGE, west_z)));
    for pair in route.windows(2) {
        expressway.push(b.road(pair[0], pair[1], RoadKind::Expressway));
    }
    drop(row_lift);
    // Independent north-south route above the first eastern interior arterial column.
    let column = RIVER_COL + 2;
    let bottom_x = b.nodes[grid[0][column]].p.x;
    let mut spur: Vec<usize> = vec![edge_node(&mut b, P::new(bottom_x, -EDGE))];
    let mut spur_elevated: BTreeMap<usize, usize> = BTreeMap::new();
    let mut spur_lift = |b: &mut Builder, street_node: usize| -> usize {
        *spur_elevated.entry(street_node).or_insert_with(|| {
            b.nodes.push(PlanNode { p: b.nodes[street_node].p, kind: NodeKind::Elevated });
            b.nodes.len() - 1
        })
    };
    for r in 0..rows.len() {
        spur.push(spur_lift(&mut b, grid[r][column]));
    }
    let top_x = b.nodes[grid[rows.len() - 1][column]].p.x;
    spur.push(edge_node(&mut b, P::new(top_x, EDGE)));
    for pair in spur.windows(2) {
        let road_index = b.road(pair[0], pair[1], RoadKind::Expressway);
        b.roads[road_index].deck_height = 20.0;
        expressway.push(road_index);
    }

    let Builder { nodes, roads, lines, .. } = b;
    let mut lots = Vec::new();
    for pending_lot in pending {
        let line = &lines[pending_lot.line];
        let (pa, pb) = (nodes[line.a].p, nodes[line.b].p);
        let d = pb.sub(pa);
        let t = pending_lot.front_mid.sub(pa).dot(d) / d.dot(d);
        let Some(road) = line
            .roads
            .iter()
            .find(|(t0, t1, _)| t >= *t0 - 1.0e-4 && t <= *t1 + 1.0e-4)
            .map(|s| s.2)
        else {
            continue;
        };
        let mut lot = pending_lot.lot;
        lot.road = road;
        let corners = rect_corners(&lot);
        let pr = geo.protected;
        if corners.iter().any(|p| {
            river_distance(river, *p) < geo.river_clearance
                || p.x.abs() > EDGE - 4.0
                || p.z.abs() > EDGE - 4.0
                || (p.z - rail_z).abs() < RAIL_BAND_HALF - 0.5
                || (p.x >= pr.0 - 2.0 && p.x <= pr.2 + 2.0 && p.z >= pr.1 - 2.0 && p.z <= pr.3 + 2.0)
        }) {
            continue;
        }
        // Every street keeps its setback, including streets that continue
        // past a block corner at an angle (bent arterials beyond a collector).
        let corners_exact = rect_corners(&lot);
        let reach = lot.width.max(lot.depth) + 12.0;
        let intrudes = roads.iter().any(|road| {
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
            quad_segment_distance(&corners_exact, a, b2) < road.kind.lot_setback() - 0.5
        });
        if intrudes {
            continue;
        }
        lots.push(lot);
        if lots.len() >= MAX_PLAN_LOTS {
            break;
        }
    }

    MasterPlan {
        nodes,
        roads,
        lots,
        blocks: block_outlines,
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
    pub(crate) fn digest(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        let mut feed = |value: u32| {
            for byte in value.to_le_bytes() {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
        };
        for node in &self.nodes {
            feed(node.p.x.to_bits());
            feed(node.p.z.to_bits());
        }
        for road in &self.roads {
            feed(road.a as u32);
            feed(road.b as u32);
            feed(road.kind as u32);
            feed(road.deck_height.to_bits());
        }
        for lot in &self.lots {
            for value in [lot.x, lot.z, lot.width, lot.depth, lot.facing.x, lot.facing.z] {
                feed(value.to_bits());
            }
            feed(lot.road as u32);
        }
        hash
    }

    /// Total lot area divided by the plan extent, as a quick density proxy.
    #[cfg(test)]
    fn lot_coverage(&self) -> f32 {
        let area: f32 = self.lots.iter().map(|l| l.width * l.depth).sum();
        area / (2.0 * PLAN_EXTENT).powi(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry;
    use std::collections::BTreeSet;

    fn geography(seed: u32) -> Geography {
        let bend = (seed % 5) as f32 * 9.0 - 18.0;
        Geography {
            seed,
            river: [
                P::new(-112.0 + bend, -400.0),
                P::new(-98.0 + bend, -190.0),
                P::new(-112.0 + bend, 20.0),
                P::new(-94.0 + bend, 210.0),
                P::new(-109.0 + bend, 400.0),
            ],
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
    fn plan_is_deterministic() {
        let a = generate(&geography(87231));
        let b = generate(&geography(87231));
        assert_eq!(a.lots, b.lots);
        assert_eq!(a.roads, b.roads);
        let c = generate(&geography(7));
        assert_ne!(a.lots, c.lots);
    }

    #[test]
    fn lots_are_disjoint_and_dense() {
        for seed in [0, 7, 41, 2026, 87231] {
            let plan = generate(&geography(seed));
            assert!(plan.lots.len() > 1_200, "seed {seed}: {} lots", plan.lots.len());
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
        let plan = generate(&geography(87231));
        for lot in &plan.lots {
            let road = plan.roads[lot.road];
            assert!(road.kind.is_street());
            let (a, b) = (plan.nodes[road.a].p, plan.nodes[road.b].p);
            let r = rect(lot);
            // The lot line sits at the road's setback, never inside it.
            let clearance = geometry::rect_segment_distance(&r, a.x, a.z, b.x, b.z);
            assert!(
                clearance >= road.kind.lot_setback() - 0.6,
                "lot intrudes on frontage road: {clearance} {lot:?}"
            );
            let reach = if lot.flag { 34.0 } else { road.kind.lot_setback() + 1.5 };
            assert!(clearance <= reach, "lot is not at its frontage: {clearance} flag={}", lot.flag);
            // Local +Z faces the road.
            let facing = P::new(lot.rotation.sin(), lot.rotation.cos());
            let mid = a.lerp(b, 0.5);
            let to_road_line = {
                let d = b.sub(a);
                let t = P::new(lot.x, lot.z).sub(a).dot(d) / d.dot(d);
                a.lerp(b, t).sub(P::new(lot.x, lot.z))
            };
            let _ = mid;
            assert!(facing.dot(to_road_line.unit()) > 0.95, "lot does not face its road");
            for other in &plan.roads {
                if !other.kind.is_street() {
                    continue;
                }
                let (oa, ob) = (plan.nodes[other.a].p, plan.nodes[other.b].p);
                let d = geometry::rect_segment_distance(&r, oa.x, oa.z, ob.x, ob.z);
                assert!(
                    d >= other.kind.lot_setback() - 0.6,
                    "lot intrudes on {:?} by {d}",
                    other.kind
                );
            }
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
            assert!(wet < 110.0, "seed {seed}: expressway runs {wet} m over the river");
        }
    }

    #[test]
    fn expressway_corridors_cross_as_separate_decks_without_a_junction() {
        let mut plan = generate(&geography(87231));
        let horizontal: Vec<usize> = plan.expressway.iter().copied()
            .filter(|&i| plan.roads[i].deck_height == 11.0)
            .collect();
        let vertical: Vec<usize> = plan.expressway.iter().copied()
            .filter(|&i| plan.roads[i].deck_height == 20.0)
            .collect();
        let horizontal_nodes: BTreeSet<usize> = horizontal.iter().flat_map(|&i| {
            let road = plan.roads[i];
            [road.a, road.b]
        }).collect();
        let vertical_nodes: BTreeSet<usize> = vertical.iter().flat_map(|&i| {
            let road = plan.roads[i];
            [road.a, road.b]
        }).collect();
        assert!(horizontal_nodes.is_disjoint(&vertical_nodes), "crossing corridors share a node");
        assert!(horizontal.iter().all(|&i| plan.roads[i].deck_height == 11.0));
        assert!(vertical.iter().all(|&i| plan.roads[i].deck_height == 20.0));
        let crosses = horizontal_nodes.iter().any(|&h| {
            vertical_nodes.iter().any(|&v| {
                let hp = plan.nodes[h].p;
                let vp = plan.nodes[v].p;
                h != v && hp.x == vp.x && hp.z == vp.z
            })
        });
        assert!(crosses, "expressway corridors should cross spatially");
        assert_eq!(generate(&geography(87231)).digest(), plan.digest());
        let previous = plan.digest();
        plan.roads[vertical[0]].deck_height = 11.0;
        assert_ne!(previous, plan.digest(), "deck changes must invalidate saved plans");
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
}

#[cfg(test)]
mod svg {
    use super::*;

    /// `PLAN_SVG=/path/plan.svg cargo test --release masterplan::svg -- --ignored`
    #[test]
    #[ignore]
    fn write_plan_svg() {
        let path = std::env::var("PLAN_SVG").unwrap_or_else(|_| "plan.svg".into());
        let seed: u32 = std::env::var("PLAN_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(87231);
        let bend = (seed % 5) as f32 * 9.0 - 18.0;
        let geo = Geography {
            seed,
            river: [
                P::new(-112.0 + bend, -400.0),
                P::new(-98.0 + bend, -190.0),
                P::new(-112.0 + bend, 20.0),
                P::new(-94.0 + bend, 210.0),
                P::new(-109.0 + bend, 400.0),
            ],
            river_clearance: 38.0,
            protected: (-350.0, -350.0, -260.0, -270.0),
        };
        let plan = generate(&geo);
        let mut s = String::from(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-400 -400 800 800" width="1000" height="1000"><rect x="-400" y="-400" width="800" height="800" fill="#e4e8dc"/>"##);
        let r = geo.river;
        s += &format!(r##"<polyline points="{}" fill="none" stroke="#86b2bb" stroke-width="64"/>"##, r.iter().map(|p| format!("{},{}", p.x, -p.z)).collect::<Vec<_>>().join(" "));
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
            s += &format!(r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{c}" stroke-width="{w}" stroke-opacity="0.85"/>"##, a.x, -a.z, b.x, -b.z);
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
            s += &format!(r##"<polygon points="{}" fill="{fill}" stroke="#e4e8dc" stroke-width="0.25"/>"##, c.iter().map(|p| format!("{:.2},{:.2}", p.x, -p.z)).collect::<Vec<_>>().join(" "));
        }
        let st = plan.nodes[plan.station].p;
        s += &format!(r##"<circle cx="{}" cy="{}" r="6" fill="#c0392b"/></svg>"##, st.x, -st.z);
        std::fs::write(path, s).unwrap();
    }
}

#[cfg(test)]
mod export {
    use super::*;

    /// `PLAN_JSON=/path PLAN_SEED=87231 cargo test --release masterplan::export -- --ignored`
    /// Exports the plan for scripts/plateau-street-reference.py.
    #[test]
    #[ignore]
    fn export_plan_json() {
        let path = std::env::var("PLAN_JSON").unwrap_or_else(|_| "plan.json".into());
        let seed: u32 = std::env::var("PLAN_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(87231);
        let bend = (seed % 5) as f32 * 9.0 - 18.0;
        let river = [
            P::new(-112.0 + bend, -400.0),
            P::new(-98.0 + bend, -190.0),
            P::new(-112.0 + bend, 20.0),
            P::new(-94.0 + bend, 210.0),
            P::new(-109.0 + bend, 400.0),
        ];
        let plan = generate(&Geography {
            seed,
            river,
            river_clearance: 38.0,
            protected: (-350.0, -350.0, -260.0, -270.0),
        });
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
        let json = serde_json::json!({
            "seed": seed,
            "extent": 400.0,
            "river": river.iter().map(|p| [p.x, p.z]).collect::<Vec<_>>(),
            "riverHalfWidth": 32.0,
            "nodes": nodes,
            "roads": roads,
            "lots": lots,
        });
        std::fs::write(path, json.to_string()).unwrap();
    }
}
