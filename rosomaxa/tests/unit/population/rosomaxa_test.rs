use super::*;
use crate::example::*;
use crate::helpers::example::create_example_objective;

type RosomaxaType = Rosomaxa<VectorRosomaxaContext, VectorObjective, VectorSolution>;

#[test]
fn can_reject_invalid_config() {
    let invalid_configs = [
        RosomaxaConfig { initial_size: 0, ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { max_network_size: 0, ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { spread_factor: 0., ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { spread_factor: 1., ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { spread_factor: Float::NAN, ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { distribution_factor: 0., ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { distribution_factor: 1., ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { distribution_factor: Float::NAN, ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { exploration_ratio: -0.1, ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { exploration_ratio: 1.1, ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { exploration_ratio: Float::NAN, ..RosomaxaConfig::new_with_defaults(4) },
    ];

    for config in invalid_configs {
        let result =
            Rosomaxa::new(VectorRosomaxaContext, create_example_objective(), Arc::new(Environment::default()), config);

        assert!(result.is_err());
    }
}

#[test]
fn can_accept_exploration_ratio_boundaries() {
    for exploration_ratio in [0., 1.] {
        let config = RosomaxaConfig { exploration_ratio, ..RosomaxaConfig::new_with_defaults(4) };
        let result =
            Rosomaxa::new(VectorRosomaxaContext, create_example_objective(), Arc::new(Environment::default()), config);

        assert!(result.is_ok());
    }
}

mod selection {
    use super::*;

    fn create_rosomaxa(initial_size: usize) -> RosomaxaType {
        create_rosomaxa_with_config(RosomaxaConfig { initial_size, ..RosomaxaConfig::new_with_defaults(4) })
    }

    fn create_rosomaxa_with_config(config: RosomaxaConfig) -> RosomaxaType {
        let env = Arc::new(Environment::default());
        let objective = create_example_objective();

        Rosomaxa::new(VectorRosomaxaContext, objective, env, config).unwrap()
    }

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

        let selected = RosomaxaType::select_initial_data(data, objective.as_ref(), 2)
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

            rosomaxa
                .on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..HeuristicStatistics::default() });

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
    fn can_track_new_exploration_inputs_and_warm_up_network() {
        let initial_size = 4;
        let mut rosomaxa = create_rosomaxa(initial_size);

        for value in 0..initial_size {
            let value = value as Float;
            rosomaxa.add(VectorSolution { data: vec![value], weights: vec![value], fitness: -value });
        }
        rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });

        let RosomaxaPhases::Exploration { new_input_count, .. } = &rosomaxa.phase else { unreachable!() };
        assert_eq!(*new_input_count, 0);

        rosomaxa.add(VectorSolution { data: vec![5.], weights: vec![5.], fitness: -5. });
        rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });
        let RosomaxaPhases::Exploration { new_input_count, .. } = &rosomaxa.phase else { unreachable!() };
        assert_eq!(*new_input_count, 1);

        let observation_count = match &rosomaxa.phase {
            RosomaxaPhases::Exploration { network, .. } => {
                network.size().max(rosomaxa.config.max_network_size.div_ceil(6))
            }
            _ => unreachable!(),
        };

        let RosomaxaPhases::Exploration { new_input_count, .. } = &mut rosomaxa.phase else { unreachable!() };
        *new_input_count = observation_count;
        rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });
        let RosomaxaPhases::Exploration { new_input_count, .. } = &rosomaxa.phase else { unreachable!() };
        assert_eq!(*new_input_count, observation_count);

        let network_size = match &rosomaxa.phase {
            RosomaxaPhases::Exploration { network, .. } => network.size(),
            _ => unreachable!(),
        };
        rosomaxa.config.max_network_size = network_size * 3;
        rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });
        let RosomaxaPhases::Exploration { new_input_count, .. } = &rosomaxa.phase else { unreachable!() };
        assert_eq!(*new_input_count, 0);
    }

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
    fn can_identify_strict_local_optimum() {
        assert!(RosomaxaType::is_strict_local_optimum([Ordering::Less, Ordering::Equal].into_iter()));
        assert!(!RosomaxaType::is_strict_local_optimum([Ordering::Greater, Ordering::Less].into_iter()));
        assert!(!RosomaxaType::is_strict_local_optimum([Ordering::Equal, Ordering::Equal].into_iter()));
        assert!(!RosomaxaType::is_strict_local_optimum(std::iter::empty()));
    }

    #[test]
    fn can_promote_local_optimum() {
        let mut coordinates = [Coordinate(0, 0), Coordinate(1, 0)];

        RosomaxaType::promote_coordinate(&mut coordinates, &DefaultRandom::new_repeatable(), |coordinate| {
            coordinate.0 == 1
        });

        assert_eq!(coordinates[0], Coordinate(1, 0));
    }

    #[test]
    fn can_select_smooth_diverse_local_optimum() {
        let mut candidates = [(1, 0.1), (2, 0.2), (3, 0.3), (4, 0.9)];
        let distances = [0., 0.2, 0.8, 0.9, 10.];
        let evaluations = std::cell::Cell::new(0);

        let selected = RosomaxaType::select_diverse_local_optimum(&mut candidates, |index| {
            evaluations.set(evaluations.get() + 1);
            distances[index]
        });

        assert_eq!(selected, Some(3));
        assert_eq!(evaluations.get(), 3);
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
}

mod auxiliary {
    use super::*;

    #[test]
    fn can_create_dedup_fn() {
        let objective = create_example_objective();
        let dedup_fn = create_dedup_fn::<VectorRosomaxaContext, _, _>(0.1);

        // test equal fitness
        let solution1 = VectorSolution { data: vec![1.0], weights: vec![1.0], fitness: -1.0 };
        let solution2 = VectorSolution { data: vec![1.0], weights: vec![1.0], fitness: -1.0 };
        assert!(dedup_fn(objective.as_ref(), &solution1, &solution2));

        // Test similar weights but different fitness
        let solution3 = VectorSolution { data: vec![1.05], weights: vec![1.05], fitness: -1.5 };
        assert!(dedup_fn(objective.as_ref(), &solution1, &solution3));

        // Test different weights
        let solution4 = VectorSolution { data: vec![2.0], weights: vec![2.0], fitness: -2.0 };
        assert!(!dedup_fn(objective.as_ref(), &solution1, &solution4));
    }

    #[test]
    fn can_get_keep_size() {
        let max_network_size = 300;

        // early phase
        let size_early = get_keep_size(max_network_size, 0.0);
        assert!(size_early > max_network_size * 2 / 3);

        // mid phase
        let size_mid = get_keep_size(max_network_size, 0.5);
        assert!(size_mid > max_network_size * 2 / 3);
        assert!(size_mid < size_early);

        // late phase
        let size_late = get_keep_size(max_network_size, 0.8);
        assert!(size_late >= max_network_size * 2 / 3);
        assert!(size_late < size_mid);

        assert_eq!(get_keep_size(4, 1.), 4);
        assert_eq!(get_min_network_size(600), 200);
        assert_eq!(get_min_network_size(4), 4);
    }

    #[test]
    fn can_get_exploration_ratio() {
        assert_eq!(get_exploration_ratio(0.9, 0.), 0.9);
        assert_eq!(get_exploration_ratio(0.9, 0.001), 0.95);
        assert_eq!(get_exploration_ratio(0., 0.001), 0.);
        assert_eq!(get_exploration_ratio(0.97, 0.001), 0.97);
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
    fn can_cool_node_alternative_probability() {
        assert_eq!(get_node_alternative_probability(0.), 0.05);
        assert_eq!(get_node_alternative_probability(0.5), 0.025);
        assert_eq!(get_node_alternative_probability(1.), 0.);
    }

    #[test]
    fn can_scale_exploitation_selection_size() {
        assert_eq!((1..=8).map(get_exploitation_selection_size).collect::<Vec<_>>(), [2, 2, 2, 2, 3, 3, 4, 4]);
        assert_eq!(get_exploitation_selection_size(16), 8);
        assert_eq!(get_exploitation_selection_size(64), 32);
    }

    #[test]
    fn can_get_learning_rate() {
        // test learning rate boundaries
        assert!(get_learning_rate(0.0) >= 0.1);
        assert!(get_learning_rate(1.0) >= 0.1);

        // test cosine annealing pattern
        let rate1 = get_learning_rate(0.0);
        let rate2 = get_learning_rate(0.125);
        let rate3 = get_learning_rate(0.25);

        // rate should decrease initially
        assert!(rate1 > rate2);
        // rate should increase towards the end of period
        assert!(rate2 < rate3);

        // test period cycling
        let rate_period1 = get_learning_rate(0.1);
        let rate_period2 = get_learning_rate(0.35);
        assert!((rate_period1 - rate_period2).abs() < 0.01);
    }
}
