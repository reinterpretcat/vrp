use super::*;
use crate::example::*;
use crate::helpers::example::create_example_objective;

type RosomaxaType = Rosomaxa<VectorRosomaxaContext, VectorObjective, VectorSolution>;

mod selection {
    use super::*;

    fn create_rosomaxa(initial_size: usize) -> RosomaxaType {
        let env = Arc::new(Environment::default());
        let config = RosomaxaConfig { initial_size, ..RosomaxaConfig::new_with_defaults(4) };
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
        let rebalance_memory = 100;

        // early phase
        let size_early = get_keep_size(rebalance_memory, 0.0);
        assert!(size_early > rebalance_memory * 2);

        // mid phase
        let size_mid = get_keep_size(rebalance_memory, 0.5);
        assert!(size_mid > rebalance_memory);
        assert!(size_mid < size_early);

        // late phase
        let size_late = get_keep_size(rebalance_memory, 0.8);
        assert!(size_late >= rebalance_memory);
        assert!(size_late < size_mid);
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
