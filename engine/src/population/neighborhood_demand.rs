//! Pure, bounded admission for residual-household errands.
//!
//! Callers provide stable-ID-sorted snapshots. `scan` does not reserve capacity
//! or persist demand; persist its returned cursor even when route validation
//! later rejects the proposal.

pub const MAX_ACTIVE_ERRANDS: u32 = 128;
pub const MAX_DESTINATION_CHECKS: usize = 8;
pub const ERRAND_START_MINUTE: u16 = 9 * 60;
pub const ERRAND_END_MINUTE: u16 = 21 * 60;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Home {
    pub id: u32,
    pub x: f32,
    pub z: f32,
    pub occupied_people: u32,
    pub named_residents: u32,
    /// Aggregate participants outbound, dwelling, or returning and still away from home.
    pub active_aggregate_participants: u32,
    pub active_outings: u32,
    pub outing_quota: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Destination {
    pub id: u32,
    pub x: f32,
    pub z: f32,
    pub open: bool,
    pub visitor_capacity: u32,
    pub current_visitors: u32,
    /// Reserved slots for accepted outbound and dwell participants.
    /// These participants must NOT also be included in `current_visitors`.
    pub reserved_visitors: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScanCursor {
    /// Next home slot in the caller's stable ID-ordered slice.
    pub home_index: u64,
    /// Destination rotation round; incremented whenever home scanning wraps.
    pub destination_round: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ErrandProposal {
    pub home_id: u32,
    pub destination_id: u32,
    pub seed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanResult {
    pub proposal: Option<ErrandProposal>,
    pub next_cursor: ScanCursor,
    pub destination_checks: u8,
}

/// Select at most one distance-preferred legal destination for the current
/// home. Each call examines one home and at most eight rotating destinations.
/// Inputs must be stable-ID-sorted. The per-home seeded start plus saved round
/// ensures destination coverage without sorting or scanning the entire city.
pub fn scan(
    homes: &[Home],
    destinations: &[Destination],
    minute_of_day: u16,
    seed: u64,
    active_aggregate_outings: u32,
    cursor: ScanCursor,
) -> ScanResult {
    let no_proposal = |next_cursor, destination_checks| ScanResult {
        proposal: None,
        next_cursor,
        destination_checks,
    };
    if homes.is_empty()
        || destinations.is_empty()
        || !(ERRAND_START_MINUTE..ERRAND_END_MINUTE).contains(&minute_of_day)
        || active_aggregate_outings >= MAX_ACTIVE_ERRANDS
    {
        return no_proposal(cursor, 0);
    }

    let home_index = (cursor.home_index % homes.len() as u64) as usize;
    let home = homes[home_index];
    let wraps = home_index + 1 == homes.len();
    let next_cursor = ScanCursor {
        home_index: if wraps { 0 } else { home_index as u64 + 1 },
        destination_round: if wraps {
            cursor.destination_round.wrapping_add(1)
        } else {
            cursor.destination_round
        },
    };
    let checks = destinations.len().min(MAX_DESTINATION_CHECKS);
    let residual = home
        .occupied_people
        .saturating_sub(home.named_residents)
        .saturating_sub(home.active_aggregate_participants);
    if residual == 0 || home.active_outings >= home.outing_quota {
        return no_proposal(next_cursor, 0);
    }

    let start = destination_start(seed, home.id, cursor.destination_round, destinations.len());
    let mut best: Option<(f32, u64, ErrandProposal)> = None;
    for offset in 0..checks {
        let destination = destinations[(start + offset) % destinations.len()];
        if home.id == destination.id
            || !destination.open
            || !home.x.is_finite()
            || !home.z.is_finite()
            || !destination.x.is_finite()
            || !destination.z.is_finite()
            || destination
                .current_visitors
                .saturating_add(destination.reserved_visitors)
                >= destination.visitor_capacity
        {
            continue;
        }
        let dx = home.x - destination.x;
        let dz = home.z - destination.z;
        let distance = dx * dx + dz * dz;
        if !distance.is_finite() {
            continue;
        }
        let proposal = ErrandProposal {
            home_id: home.id,
            destination_id: destination.id,
            seed,
        };
        let tie =
            mix(seed ^ (home.id as u64).rotate_left(17) ^ (destination.id as u64).rotate_left(39));
        if best.map_or(true, |(d, t, _)| distance < d || (distance == d && tie < t)) {
            best = Some((distance, tie, proposal));
        }
    }
    ScanResult {
        proposal: best.map(|(_, _, proposal)| proposal),
        next_cursor,
        destination_checks: checks as u8,
    }
}

/// Reduce before casting: WASM uses 32-bit usize while native uses 64-bit.
fn destination_start(seed: u64, home_id: u32, round: u64, count: usize) -> usize {
    let count = count as u64;
    let base = mix(seed ^ (home_id as u64).rotate_left(19)) % count;
    // Slice lengths are bounded by addressable memory. Use u128 for modular
    // arithmetic so even restored cursors near u64::MAX cannot overflow.
    ((base as u128 + round as u128 * MAX_DESTINATION_CHECKS as u128) % count as u128) as usize
}

fn mix(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(id: u32, occupancy: u32) -> Home {
        Home {
            id,
            x: 0.0,
            z: 0.0,
            occupied_people: occupancy,
            named_residents: 0,
            active_aggregate_participants: 0,
            active_outings: 0,
            outing_quota: 4,
        }
    }
    fn dest(id: u32, x: f32) -> Destination {
        Destination {
            id,
            x,
            z: 0.0,
            open: true,
            visitor_capacity: 10,
            current_visitors: 0,
            reserved_visitors: 0,
        }
    }

    #[test]
    fn hash_selection_reduces_before_pointer_width_conversion() {
        let seed = 42;
        let hash = mix(seed ^ 1_u64.rotate_left(19));
        assert_ne!(hash % 100, (hash as u32 as u64) % 100);
        assert_eq!(destination_start(seed, 1, 0, 100), (hash % 100) as usize);
        assert_eq!(
            destination_start(seed, 1, u64::MAX, 100),
            ((hash as u128 % 100 + u64::MAX as u128 * 8) % 100) as usize
        );
    }

    #[test]
    fn rotating_shortlist_eventually_reaches_every_destination() {
        // Only one destination has capacity. Every possible position must
        // become selectable; successful first-page scans alone cannot prove it.
        for count in [1, 7, 8, 9, 16, 100] {
            for target in 0..count {
                let destinations: Vec<_> = (0..count)
                    .map(|i| {
                        let mut d = dest(20 + i as u32, 1.0);
                        d.open = i == target;
                        d
                    })
                    .collect();
                let mut cursor = ScanCursor::default();
                let mut found = false;
                for _ in 0..count {
                    let result = scan(&[home(1, 1)], &destinations, 900, 42, 0, cursor);
                    assert!(result.destination_checks as usize <= MAX_DESTINATION_CHECKS);
                    cursor = result.next_cursor;
                    if let Some(proposal) = result.proposal {
                        assert_eq!(proposal.destination_id, 20 + target as u32);
                        found = true;
                        break;
                    }
                }
                assert!(found, "unreachable destination {target} of {count}");
            }
        }
    }

    #[test]
    fn subtracts_named_and_active_participants_once() {
        let mut h = home(1, 6);
        h.named_residents = 3;
        h.active_aggregate_participants = 2;
        let result = scan(&[h], &[dest(20, 1.0)], 900, 7, 2, ScanCursor::default());
        assert!(result.proposal.is_some()); // Exactly one residual person remains.
    }
    #[test]
    fn active_participant_depletes_last_residual_person() {
        let mut h = home(1, 4);
        h.named_residents = 2;
        h.active_aggregate_participants = 2;
        assert!(
            scan(&[h], &[dest(20, 1.0)], 900, 0, 2, ScanCursor::default())
                .proposal
                .is_none()
        );
    }
    #[test]
    fn zero_residual_has_no_proposal() {
        let mut h = home(1, 2);
        h.named_residents = 2;
        assert!(
            scan(&[h], &[dest(20, 1.0)], 900, 0, 0, ScanCursor::default())
                .proposal
                .is_none()
        );
    }
    #[test]
    fn visitor_capacity_includes_reserved_outbound_and_dwell() {
        let mut d = dest(20, 1.0);
        d.visitor_capacity = 4;
        d.current_visitors = 2;
        d.reserved_visitors = 2;
        assert!(scan(&[home(1, 1)], &[d], 900, 0, 0, ScanCursor::default())
            .proposal
            .is_none());
    }
    #[test]
    fn window_is_start_inclusive_and_end_exclusive() {
        let hs = [home(1, 1)];
        let ds = [dest(20, 1.0)];
        assert!(scan(&hs, &ds, 539, 0, 0, ScanCursor::default())
            .proposal
            .is_none());
        assert!(scan(&hs, &ds, 540, 0, 0, ScanCursor::default())
            .proposal
            .is_some());
        assert!(scan(&hs, &ds, 1260, 0, 0, ScanCursor::default())
            .proposal
            .is_none());
    }
    #[test]
    fn destination_scan_is_bounded_and_resume_is_deterministic() {
        let hs = [home(1, 1)];
        let ds: Vec<_> = (20..120).map(|i| dest(i, i as f32)).collect();
        let a = scan(&hs, &ds, 900, 42, 0, ScanCursor::default());
        assert_eq!(a.destination_checks, 8);
        assert_eq!(a.next_cursor.destination_round, 1);
        assert_eq!(a, scan(&hs, &ds, 900, 42, 0, ScanCursor::default()));
    }
    #[test]
    fn round_robin_gives_each_home_a_chance_with_many_destinations() {
        let homes = [home(1, 1), home(2, 1), home(3, 1), home(4, 1)];
        let destinations: Vec<_> = (20..120).map(|i| dest(i, i as f32)).collect();
        let mut cursor = ScanCursor::default();
        let mut visited = Vec::new();
        for _ in 0..homes.len() {
            let result = scan(&homes, &destinations, 900, 123, 0, cursor);
            visited.push(result.proposal.unwrap().home_id);
            cursor = result.next_cursor;
        }
        assert_eq!(visited, vec![1, 2, 3, 4]);
    }
    #[test]
    fn closed_same_id_and_saturated_quotas_are_rejected() {
        let mut h = home(1, 1);
        let mut d = dest(1, 1.0);
        assert!(scan(&[h], &[d], 900, 0, 0, ScanCursor::default())
            .proposal
            .is_none());
        d.id = 2;
        d.open = false;
        assert!(scan(&[h], &[d], 900, 0, 0, ScanCursor::default())
            .proposal
            .is_none());
        d.open = true;
        h.active_outings = h.outing_quota;
        assert!(scan(&[h], &[d], 900, 0, 0, ScanCursor::default())
            .proposal
            .is_none());
        h.active_outings = 0;
        assert!(scan(
            &[h],
            &[d],
            900,
            0,
            MAX_ACTIVE_ERRANDS,
            ScanCursor::default()
        )
        .proposal
        .is_none());
    }
}
