//! Pure interval subdivision for bounded road frontage infill.
//!
//! The input exclusions are already-buffered centerline intervals. This
//! module has no world, geometry, or random-state dependency so callers can
//! reuse it with deterministic frontage projections.

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Interval {
    pub(crate) start: f32,
    pub(crate) end: f32,
}

const MAX_FRONTAGE_LOTS: usize = 64;
const EPSILON: f32 = 1.0e-5;

/// Subtract forbidden frontage ranges and tile the remaining spans with
/// bounded, evenly-sized parcel intervals. A final partial span is left
/// unused when it cannot be tiled without violating the minimum width.
pub(crate) fn subdivide_frontage(
    length: f32,
    exclusions: &[Interval],
    endpoint_margin: f32,
    minimum_width: f32,
    target_width: f32,
    gap: f32,
    max_lots: usize,
) -> Vec<Interval> {
    if !length.is_finite()
        || !endpoint_margin.is_finite()
        || !minimum_width.is_finite()
        || !target_width.is_finite()
        || !gap.is_finite()
        || length <= 0.0
        || endpoint_margin < 0.0
        || endpoint_margin * 2.0 >= length
        || minimum_width <= 0.0
        || target_width < minimum_width
        || gap < 0.0
        || max_lots == 0
        || exclusions
            .iter()
            .any(|item| !item.start.is_finite() || !item.end.is_finite() || item.start > item.end)
    {
        return Vec::new();
    }

    let usable_start = endpoint_margin;
    let usable_end = length - endpoint_margin;
    let mut forbidden: Vec<Interval> = exclusions
        .iter()
        .filter_map(|item| {
            let start = item.start.max(usable_start);
            let end = item.end.min(usable_end);
            (end > start).then_some(Interval { start, end })
        })
        .collect();
    forbidden.sort_by(|a, b| {
        a.start
            .total_cmp(&b.start)
            .then_with(|| a.end.total_cmp(&b.end))
    });

    let mut merged: Vec<Interval> = Vec::with_capacity(forbidden.len());
    for item in forbidden {
        if let Some(last) = merged.last_mut() {
            if item.start <= last.end {
                last.end = last.end.max(item.end);
                continue;
            }
        }
        merged.push(item);
    }

    let mut free_spans = Vec::with_capacity(merged.len() + 1);
    let mut cursor = usable_start;
    for item in merged {
        if item.start > cursor {
            free_spans.push(Interval {
                start: cursor,
                end: item.start,
            });
        }
        cursor = cursor.max(item.end);
    }
    if cursor < usable_end {
        free_spans.push(Interval {
            start: cursor,
            end: usable_end,
        });
    }

    let lot_limit = max_lots.min(MAX_FRONTAGE_LOTS);
    let mut lots = Vec::with_capacity(lot_limit.min(16));
    for span in free_spans {
        let span_length = span.end - span.start;
        if span_length + EPSILON < minimum_width {
            continue;
        }
        let remaining = lot_limit - lots.len();
        if remaining == 0 {
            break;
        }

        let target_count = ((span_length + gap) / (target_width + gap)).ceil() as usize;
        let target_count = target_count.max(1);
        let balanced_width =
            (span_length - gap * target_count.saturating_sub(1) as f32) / target_count as f32;

        if target_count <= remaining && balanced_width + EPSILON >= minimum_width {
            let mut start = span.start;
            for index in 0..target_count {
                let end = if index + 1 == target_count {
                    span.end
                } else {
                    (start + balanced_width).min(span.end)
                };
                if end - start + EPSILON < minimum_width {
                    break;
                }
                lots.push(Interval { start, end });
                start = end + gap;
            }
            continue;
        }

        // Do not stretch a capped set across the rest of the frontage. Emit
        // target-width lots from this span only, then stop when the cap fills.
        let fit_count = ((span_length + gap) / (target_width + gap)).floor() as usize;
        let emit_count = remaining.min(fit_count);
        if emit_count == 0 {
            continue;
        }
        let mut start = span.start;
        for _ in 0..emit_count {
            let end = start + target_width;
            if end > span.end + EPSILON {
                break;
            }
            lots.push(Interval {
                start,
                end: end.min(span.end),
            });
            start = end + gap;
        }
        if lots.len() == lot_limit {
            break;
        }
    }

    lots
}

#[cfg(test)]
mod tests {
    use super::{subdivide_frontage, Interval};

    fn intervals(length: f32, exclusions: &[Interval], max_lots: usize) -> Vec<Interval> {
        subdivide_frontage(length, exclusions, 0.0, 8.0, 14.0, 2.0, max_lots)
    }

    #[test]
    fn exclusions_are_clipped_unioned_and_input_order_independent() {
        let exclusions = [
            Interval {
                start: 18.0,
                end: 29.0,
            },
            Interval {
                start: -5.0,
                end: 12.0,
            },
            Interval {
                start: 8.0,
                end: 21.0,
            },
            Interval {
                start: 29.0,
                end: 32.0,
            },
        ];
        let expected = intervals(100.0, &exclusions, 64);
        let reversed = exclusions.into_iter().rev().collect::<Vec<_>>();
        assert_eq!(expected, intervals(100.0, &reversed, 64));
        assert_eq!(
            expected,
            vec![
                Interval {
                    start: 32.0,
                    end: 44.0
                },
                Interval {
                    start: 46.0,
                    end: 58.0
                },
                Interval {
                    start: 60.0,
                    end: 72.0
                },
                Interval {
                    start: 74.0,
                    end: 86.0
                },
                Interval {
                    start: 88.0,
                    end: 100.0
                },
            ]
        );
    }

    #[test]
    fn endpoint_margins_and_exact_touches_are_respected() {
        let lots = subdivide_frontage(
            50.0,
            &[
                Interval {
                    start: 5.0,
                    end: 10.0,
                },
                Interval {
                    start: 20.0,
                    end: 25.0,
                },
                Interval {
                    start: 40.0,
                    end: 45.0,
                },
            ],
            5.0,
            8.0,
            14.0,
            2.0,
            64,
        );
        assert_eq!(
            lots,
            vec![
                Interval {
                    start: 10.0,
                    end: 20.0
                },
                Interval {
                    start: 25.0,
                    end: 39.0
                },
            ]
        );
    }

    #[test]
    fn short_spans_obey_minimum_and_long_spans_tile_with_balanced_widths() {
        let lots = intervals(
            74.0,
            &[
                Interval {
                    start: 17.0,
                    end: 23.0,
                },
                Interval {
                    start: 55.0,
                    end: 57.0,
                },
            ],
            64,
        );
        assert_eq!(lots.len(), 5);
        for lot in &lots {
            assert!(lot.end - lot.start >= 8.0 - 1e-5);
            assert!(lot.end - lot.start <= 14.0 + 1e-5);
        }
        assert_eq!(
            lots[0],
            Interval {
                start: 0.0,
                end: 14.0
            }
        );
        assert_eq!(
            lots[1],
            Interval {
                start: 23.0,
                end: 32.333_332
            }
        );
        assert_eq!(
            lots[2],
            Interval {
                start: 34.333_332,
                end: 43.666_664
            }
        );
        assert_eq!(
            lots[3],
            Interval {
                start: 45.666_664,
                end: 55.0
            }
        );
        assert_eq!(
            lots[4],
            Interval {
                start: 57.0,
                end: 71.0
            }
        );
    }

    #[test]
    fn lot_cap_never_stretches_lots_or_continues_into_later_spans() {
        let lots = intervals(
            120.0,
            &[Interval {
                start: 50.0,
                end: 60.0,
            }],
            2,
        );
        assert_eq!(lots.len(), 2);
        assert_eq!(lots[0].end - lots[0].start, 14.0);
        assert_eq!(lots[1].end - lots[1].start, 14.0);
        assert_eq!(lots[1].end, 30.0);

        let bounded = intervals(2_000.0, &[], 200);
        assert_eq!(bounded.len(), 64);
        assert!(bounded.iter().all(|lot| lot.end - lot.start <= 14.0));
    }

    #[test]
    fn invalid_dimensions_and_nonfinite_exclusions_fail_closed() {
        assert!(subdivide_frontage(20.0, &[], 10.0, 8.0, 14.0, 2.0, 10).is_empty());
        assert!(subdivide_frontage(20.0, &[], 0.0, 0.0, 14.0, 2.0, 10).is_empty());
        assert!(subdivide_frontage(20.0, &[], 0.0, 9.0, 8.0, 2.0, 10).is_empty());
        assert!(subdivide_frontage(20.0, &[], 0.0, 8.0, 14.0, -1.0, 10).is_empty());
        assert!(intervals(f32::NAN, &[], 10).is_empty());
        assert!(intervals(
            20.0,
            &[Interval {
                start: f32::INFINITY,
                end: 10.0
            }],
            10
        )
        .is_empty());
        assert!(intervals(
            20.0,
            &[Interval {
                start: 12.0,
                end: 10.0
            }],
            10
        )
        .is_empty());
        assert!(intervals(20.0, &[], 0).is_empty());
    }
}
