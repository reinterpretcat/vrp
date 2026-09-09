use super::*;

#[test]
fn can_fill_different_exploration_selection_budgets() {
    for (selection_size, node_size) in [(2, 1), (4, 2), (8, 2), (16, 4)] {
        let initial_size = 16;
        let config = RosomaxaConfig {
            initial_size,
            selection_size,
            node_size,
            ..RosomaxaConfig::new_with_defaults(selection_size)
        };
        let mut rosomaxa = create_rosomaxa_with_config(config);

        for value in 0..initial_size {
            let value = value as Float;
            rosomaxa.add(VectorSolution { data: vec![value], weights: vec![value], fitness: -value });
        }

        rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });

        assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploration);
        assert_eq!(rosomaxa.select().count(), selection_size);
    }
}

#[test]
fn can_identify_basin_sink() {
    let coordinate = Coordinate(0, 0);

    assert!(is_basin_sink(
        &coordinate,
        [(Coordinate(1, 0), Ordering::Less), (Coordinate(0, 1), Ordering::Equal)].into_iter()
    ));
    assert!(!is_basin_sink(&coordinate, [(Coordinate(1, 0), Ordering::Greater)].into_iter()));
    assert!(!is_basin_sink(&coordinate, [(Coordinate(-1, 0), Ordering::Equal)].into_iter()));
    assert!(is_basin_sink(&coordinate, std::iter::empty()));
}

#[test]
fn can_prioritize_basins_and_keep_coverage_slot() {
    let mut coordinates = (0..8).map(|idx| Coordinate(idx, 0)).collect::<Vec<_>>();
    let basins = [5, 6, 7, 4, 3].map(|idx| BasinCandidate::new(Coordinate(idx, 0)));

    promote_basin_coordinates(&mut coordinates, &basins, 6, 1);

    assert_eq!(coordinates[..6], [5, 6, 7, 0, 4, 3].map(|idx| Coordinate(idx, 0)));
}

#[test]
fn can_distribute_multiple_coverage_slots() {
    let mut coordinates = (0..12).map(|idx| Coordinate(idx, 0)).collect::<Vec<_>>();
    let basins = (3..12).rev().map(|idx| BasinCandidate::new(Coordinate(idx, 0))).collect::<Vec<_>>();

    promote_basin_coordinates(&mut coordinates, &basins, 12, 3);

    let coverage = [coordinates[2], coordinates[6], coordinates[10]];
    let prioritized = coordinates[..12]
        .iter()
        .enumerate()
        .filter(|(idx, _)| ![2, 6, 10].contains(idx))
        .map(|(_, coordinate)| *coordinate)
        .collect::<Vec<_>>();
    assert!(coverage.iter().all(|coordinate| basins.iter().all(|candidate| candidate.coordinate != *coordinate)));
    assert_eq!(prioritized, basins.iter().map(|candidate| candidate.coordinate).collect::<Vec<_>>());
}

#[test]
fn can_select_structurally_different_basins() {
    let mut candidates = [0, 1, 3, 10].map(|idx| BasinCandidate::new(Coordinate(idx, 0)));
    let evaluations = std::cell::Cell::new(0);

    select_diverse_basin_candidates(&mut candidates, 3, 0, |left, right| {
        evaluations.set(evaluations.get() + 1);
        (left.0 - right.0).abs() as Float
    });

    assert_eq!(
        candidates[..3].iter().map(|candidate| candidate.coordinate).collect::<Vec<_>>(),
        [0, 10, 3].map(|idx| Coordinate(idx, 0))
    );
    assert_eq!(evaluations.get(), 5);
}

#[test]
fn can_keep_incremental_basin_selection_equivalent() {
    let coordinates =
        [Coordinate(0, 0), Coordinate(1, 3), Coordinate(2, 1), Coordinate(4, 5), Coordinate(7, 2), Coordinate(9, 8)];
    let distance = |left: &Coordinate, right: &Coordinate| {
        let dx = (left.0 - right.0) as Float;
        let dy = (left.1 - right.1) as Float;
        dx.hypot(dy)
    };

    for selected_size in 1..=coordinates.len() {
        for reference_idx in 0..coordinates.len() {
            let mut expected = coordinates;
            expected.swap(0, reference_idx);
            for selected_idx in 1..selected_size {
                let candidate_idx = (selected_idx..expected.len())
                    .map(|candidate_idx| {
                        let min_distance = expected[..selected_idx]
                            .iter()
                            .map(|selected| distance(&expected[candidate_idx], selected))
                            .min_by(Float::total_cmp)
                            .unwrap_or_default();
                        (candidate_idx, min_distance)
                    })
                    .max_by(|(_, left), (_, right)| left.total_cmp(right))
                    .map(|(idx, _)| idx)
                    .unwrap();
                expected.swap(selected_idx, candidate_idx);
            }

            let mut actual = coordinates.map(BasinCandidate::new);
            select_diverse_basin_candidates(&mut actual, selected_size, reference_idx, distance);

            assert_eq!(
                actual[..selected_size].iter().map(|candidate| candidate.coordinate).collect::<Vec<_>>(),
                expected[..selected_size]
            );
        }
    }
}

#[test]
fn can_scale_elite_selection_size() {
    for selection_size in 2..=6 {
        assert_eq!(get_elite_selection_size(selection_size, 0., |_| true), 1);
    }

    assert_eq!(get_elite_selection_size(8, 0., |_| false), 2);
    assert_eq!(get_elite_selection_size(8, 0., |_| true), 4);
    assert_eq!(get_elite_selection_size(16, 0., |_| false), 4);
    assert_eq!(get_elite_selection_size(16, 0., |_| true), 8);
}

#[test]
fn can_keep_enough_quality_gated_basins_for_selection() {
    assert_eq!(get_basin_candidate_size(0, 4), 0);
    assert_eq!(get_basin_candidate_size(3, 4), 3);
    assert_eq!(get_basin_candidate_size(10, 4), 5);
    assert_eq!(get_basin_candidate_size(20, 4), 10);
}

#[test]
fn can_reserve_coverage_for_different_selection_sizes() {
    assert_eq!((1..=8).map(get_coverage_selection_size).collect::<Vec<_>>(), [0, 1, 1, 1, 1, 1, 1, 2]);
    assert_eq!(get_coverage_selection_size(12), 3);
}

#[test]
fn can_reserve_periodic_full_map_generation() {
    assert_eq!(
        (1..=8).map(is_basin_selection_generation).collect::<Vec<_>>(),
        [true, true, true, false, true, true, true, false]
    );
    assert_eq!(
        (1..=8).map(|generation| is_basin_shoulder_selection_generation(generation, 0.2)).collect::<Vec<_>>(),
        [true, false, false, false, false, false, false, false]
    );
    assert!(is_basin_shoulder_selection_generation(17, 0.2));
    assert_eq!(
        (1..=8).map(|generation| is_basin_shoulder_selection_generation(generation, 0.1)).collect::<Vec<_>>(),
        [true, false, false, false, true, false, false, false]
    );
}

#[test]
fn can_cool_node_alternative_probability() {
    assert_eq!(get_node_alternative_probability(0.), 0.05);
    assert_eq!(get_node_alternative_probability(0.5), 0.025);
    assert_eq!(get_node_alternative_probability(1.), 0.);
}
