use super::*;

#[test]
fn can_handle_initial_population() {
    let initial_size = 4;
    let selection_size = 4;
    let elite_size = 2;
    let mut rosomaxa = create_rosomaxa(initial_size);

    // Add initial solutions
    for i in 0..initial_size {
        let solution = VectorSolution { data: vec![i as Float], weights: vec![i as Float], fitness: -(i as Float) };
        rosomaxa.add(solution);
    }

    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Initial);
    assert_eq!(rosomaxa.select().count(), selection_size);
    assert_eq!(rosomaxa.size(), elite_size);
}

#[test]
fn can_limit_initial_selection_to_best_parents() {
    let selection_size = 4;
    let mut rosomaxa = create_rosomaxa_with_config(RosomaxaConfig {
        initial_size: 16,
        selection_size,
        ..RosomaxaConfig::new_with_defaults(selection_size)
    });

    for fitness in (0..8).rev() {
        let fitness = fitness as Float;
        rosomaxa.add(VectorSolution { data: vec![fitness], weights: vec![fitness], fitness });
    }

    let selected = rosomaxa.select().map(|solution| solution.fitness).collect::<Vec<_>>();

    assert_eq!(selected, vec![0., 1., 2., 3.]);
}

#[test]
fn can_select_quality_diverse_initial_data() {
    let objective = create_example_objective();
    let data = [
        (0., 1.),
        (1., 1.1),
        (2., 10.),
        // The structurally most distant solution is too weak to shape the initial map.
        (100., 1_000.),
    ]
    .into_iter()
    .map(|(fitness, weight)| VectorSolution { data: vec![weight], weights: vec![weight], fitness })
    .collect();

    let selected = select_initial_data(data, objective.as_ref(), 2)
        .into_iter()
        .map(|solution| solution.fitness)
        .collect::<Vec<_>>();

    assert_eq!(selected, vec![0., 2.]);
}

#[test]
fn can_handle_less_than_four_initial_solutions() {
    for initial_size in 1..4 {
        let mut rosomaxa = create_rosomaxa(initial_size);

        for i in 1..=initial_size {
            let value = i as Float;
            rosomaxa.add(VectorSolution { data: vec![value], weights: vec![value], fitness: -value });
        }

        rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..HeuristicStatistics::default() });

        assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploration);
    }
}

#[test]
fn can_handle_exploration_phase() {
    let initial_size = 4;
    let selection_size = 4;
    let mut rosomaxa = create_rosomaxa(initial_size);

    // Add solutions to trigger exploration phase
    for i in 0..=initial_size {
        let solution = VectorSolution { data: vec![i as Float], weights: vec![i as Float], fitness: -(i as Float) };
        rosomaxa.add(solution);
    }

    // Force exploration phase
    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..HeuristicStatistics::default() });
    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploration);
    assert_eq!(rosomaxa.select().count(), selection_size);
}

#[test]

fn can_handle_exploitation_phase() {
    let initial_size = 4;
    let selection_size = 4;
    let mut rosomaxa = create_rosomaxa(initial_size);

    // Add initial solutions
    for i in 0..initial_size {
        let solution = VectorSolution { data: vec![i as Float], weights: vec![i as Float], fitness: -(i as Float) };
        rosomaxa.add(solution);
    }

    // Force exploitation phase
    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.95, ..HeuristicStatistics::default() });

    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploitation);
    assert_eq!(rosomaxa.select().count(), selection_size);
}

#[test]
fn can_skip_exploration_at_exact_phase_boundary() {
    let initial_size = 4;
    let mut rosomaxa = create_rosomaxa(initial_size);

    for i in 0..initial_size {
        let solution = VectorSolution { data: vec![i as Float], weights: vec![i as Float], fitness: -(i as Float) };
        rosomaxa.add(solution);
    }

    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.9, ..HeuristicStatistics::default() });

    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploitation);
}

#[test]
fn can_extend_exploration_while_improving() {
    let initial_size = 4;
    let mut rosomaxa = create_rosomaxa(initial_size);

    for i in 0..initial_size {
        let solution = VectorSolution { data: vec![i as Float], weights: vec![i as Float], fitness: -(i as Float) };
        rosomaxa.add(solution);
    }

    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..HeuristicStatistics::default() });
    rosomaxa.on_generation(&HeuristicStatistics {
        termination_estimate: 0.9,
        improvement_1000_ratio: 0.001,
        ..HeuristicStatistics::default()
    });

    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploration);

    rosomaxa.on_generation(&HeuristicStatistics {
        termination_estimate: 0.95,
        improvement_1000_ratio: 0.001,
        ..HeuristicStatistics::default()
    });

    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploitation);
}

#[test]
fn can_fallback_to_exploitation_when_network_creation_fails() {
    let initial_size = 4;
    let mut rosomaxa = create_rosomaxa(initial_size);

    for i in 0..initial_size {
        let weights = if i == 0 { vec![i as Float] } else { vec![i as Float, i as Float] };
        rosomaxa.add(VectorSolution { data: weights.clone(), weights, fitness: -(i as Float) });
    }

    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..HeuristicStatistics::default() });

    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploitation);
    assert!(rosomaxa.select().next().is_some());
}

#[test]
fn can_keep_phase_boundary_when_speed_is_slow() {
    let initial_size = 4;
    let mut rosomaxa = create_rosomaxa(initial_size);

    for i in 0..initial_size {
        let solution = VectorSolution { data: vec![i as Float], weights: vec![i as Float], fitness: -(i as Float) };
        rosomaxa.add(solution);
    }

    let slow_speed = HeuristicSpeed::Slow { ratio: 0.1, average: 1., median: Some(1000) };
    rosomaxa.on_generation(&HeuristicStatistics {
        termination_estimate: 0.5,
        speed: slow_speed.clone(),
        ..HeuristicStatistics::default()
    });

    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploration);
    assert_eq!(rosomaxa.select().count(), 1);

    rosomaxa.on_generation(&HeuristicStatistics {
        termination_estimate: 0.9,
        speed: slow_speed,
        ..HeuristicStatistics::default()
    });

    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploitation);
}

#[test]
fn can_handle_all_phases() {
    let initial_size = 4;
    let selection_size = 4;
    let mut rosomaxa = create_rosomaxa(initial_size);

    // initial phase
    for i in 0..(initial_size - 1) {
        let solution = VectorSolution { data: vec![i as Float], weights: vec![i as Float], fitness: -(i as Float) };
        rosomaxa.add(solution);
        rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0., ..HeuristicStatistics::default() });
        assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Initial);
        assert_eq!(rosomaxa.select().count(), selection_size.min(i + 1));
    }

    // exploration phase
    rosomaxa.add(VectorSolution {
        data: vec![initial_size as Float],
        weights: vec![initial_size as Float],
        fitness: 0.,
    });
    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..HeuristicStatistics::default() });
    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploration);
    assert_eq!(rosomaxa.select().count(), selection_size);

    // stays once in exploration and switches to exploitation
    for (termination_estimate, phase) in
        [(0.7, SelectionPhase::Exploration), (0.9, SelectionPhase::Exploitation)].into_iter()
    {
        rosomaxa.on_generation(&HeuristicStatistics { termination_estimate, ..HeuristicStatistics::default() });
        assert_eq!(rosomaxa.selection_phase(), phase);
        assert_eq!(rosomaxa.select().count(), selection_size);
    }
}

#[test]
fn can_handle_empty_population() {
    let initial_size = 4;
    let mut rosomaxa = create_rosomaxa(initial_size);

    // here we're stays in initial phase for long time and go directly to exploitation
    // as we're lacking solutions for exploration
    for (phase, termination_estimate) in [
        (SelectionPhase::Initial, None),
        (SelectionPhase::Initial, Some(0.7)),
        (SelectionPhase::Exploitation, Some(0.95)),
    ] {
        if let Some(termination_estimate) = termination_estimate {
            rosomaxa.on_generation(&HeuristicStatistics { termination_estimate, ..HeuristicStatistics::default() });
        }

        assert!(rosomaxa.select().next().is_none());
        assert_eq!(rosomaxa.selection_phase(), phase)
    }
}

#[test]
fn can_handle_solution_deduplication() {
    let initial_size = 4;
    let mut rosomaxa = create_rosomaxa(initial_size);

    // Add duplicate solutions
    let solution = VectorSolution { data: vec![1.0], weights: vec![], fitness: -1.0 };

    rosomaxa.add(solution.clone());
    rosomaxa.add(solution);

    assert_eq!(rosomaxa.size(), 1);
}

#[test]
fn can_get_exploration_ratio() {
    assert_eq!(get_exploration_ratio(0.9, 0.), 0.9);
    assert_eq!(get_exploration_ratio(0.9, 0.001), 0.95);
    assert_eq!(get_exploration_ratio(0., 0.001), 0.);
    assert_eq!(get_exploration_ratio(0.97, 0.001), 0.97);
}

#[test]
fn can_scale_exploitation_selection_size() {
    assert_eq!((1..=8).map(get_exploitation_selection_size).collect::<Vec<_>>(), [2, 2, 2, 2, 3, 3, 4, 4]);
    assert_eq!(get_exploitation_selection_size(16), 8);
    assert_eq!(get_exploitation_selection_size(64), 32);
}
