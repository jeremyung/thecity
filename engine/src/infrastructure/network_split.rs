//! Pure route expansion used when an existing road edge gains a junction.
//!
//! The original edge remains geometrically straight. Inserting the split node
//! therefore preserves cumulative route distance and actor position.

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ExpandedRoute {
    pub route: Vec<u32>,
    /// New edge index corresponding to the start of each original edge.
    pub edge_starts: Vec<usize>,
    /// Length of the first new edge when the original edge was split.
    pub first_split_lengths: Vec<Option<f32>>,
}

pub(crate) fn expand_route(
    route: &[u32],
    a: u32,
    b: u32,
    split_node: u32,
    a_to_split: f32,
    split_to_b: f32,
) -> ExpandedRoute {
    let mut expanded = Vec::with_capacity(route.len().saturating_add(2));
    let mut edge_starts = Vec::with_capacity(route.len().saturating_sub(1));
    let mut first_split_lengths = Vec::with_capacity(route.len().saturating_sub(1));
    if let Some(first) = route.first() {
        expanded.push(*first);
    }

    for pair in route.windows(2) {
        edge_starts.push(expanded.len().saturating_sub(1));
        if pair[0] == a && pair[1] == b {
            expanded.push(split_node);
            expanded.push(b);
            first_split_lengths.push(Some(a_to_split));
        } else if pair[0] == b && pair[1] == a {
            expanded.push(split_node);
            expanded.push(a);
            first_split_lengths.push(Some(split_to_b));
        } else {
            expanded.push(pair[1]);
            first_split_lengths.push(None);
        }
    }

    ExpandedRoute {
        route: expanded,
        edge_starts,
        first_split_lengths,
    }
}

pub(crate) fn remap_edge_position(
    expanded: &ExpandedRoute,
    old_edge_count: usize,
    old_edge_index: usize,
    old_edge_progress: f32,
) -> Option<(usize, f32)> {
    if old_edge_index > old_edge_count || expanded.edge_starts.len() != old_edge_count {
        return None;
    }
    if old_edge_index == old_edge_count {
        return Some((expanded.route.len().saturating_sub(1), old_edge_progress));
    }

    let start = *expanded.edge_starts.get(old_edge_index)?;
    match expanded.first_split_lengths.get(old_edge_index)?.as_ref() {
        Some(first_length) if old_edge_progress >= *first_length => {
            Some((start.saturating_add(1), old_edge_progress - *first_length))
        }
        _ => Some((start, old_edge_progress)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_forward_reverse_and_repeated_edges_with_stable_indices() {
        let expanded = expand_route(&[10, 20, 30, 20, 10], 10, 20, 99, 12.0, 8.0);
        assert_eq!(expanded.route, [10, 99, 20, 30, 20, 99, 10]);
        assert_eq!(expanded.edge_starts, [0, 2, 3, 4]);
        assert_eq!(
            expanded.first_split_lengths,
            [Some(12.0), None, None, Some(8.0)]
        );
        assert_eq!(remap_edge_position(&expanded, 4, 0, 7.0), Some((0, 7.0)));
        assert_eq!(remap_edge_position(&expanded, 4, 0, 12.0), Some((1, 0.0)));
        assert_eq!(remap_edge_position(&expanded, 4, 3, 6.0), Some((4, 6.0)));
        assert_eq!(remap_edge_position(&expanded, 4, 3, 8.0), Some((5, 0.0)));
        assert_eq!(remap_edge_position(&expanded, 4, 2, 3.0), Some((3, 3.0)));
        assert_eq!(remap_edge_position(&expanded, 4, 4, 0.0), Some((6, 0.0)));
        assert_eq!(remap_edge_position(&expanded, 4, 5, 0.0), None);
    }

    #[test]
    fn leaves_routes_without_host_edge_unchanged() {
        let expanded = expand_route(&[1, 2, 3], 10, 20, 99, 5.0, 5.0);
        assert_eq!(expanded.route, [1, 2, 3]);
        assert_eq!(expanded.edge_starts, [0, 1]);
        assert_eq!(expanded.first_split_lengths, [None, None]);
    }
}
