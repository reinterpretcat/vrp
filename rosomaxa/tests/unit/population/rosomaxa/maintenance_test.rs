use super::*;

#[test]
fn can_track_exploration_inputs_and_refresh_young_network() {
    let initial_size = 4;
    let mut rosomaxa = create_rosomaxa(initial_size);

    for value in 0..initial_size {
        let value = value as Float;
        rosomaxa.add(VectorSolution { data: vec![value], weights: vec![value], fitness: -value });
    }
    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });

    let RosomaxaPhases::Exploration { maintenance, .. } = &rosomaxa.phase else { unreachable!() };
    assert_eq!(maintenance.new_input_count, 0);

    rosomaxa.add(VectorSolution { data: vec![5.], weights: vec![5.], fitness: -5. });
    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });
    let RosomaxaPhases::Exploration { maintenance, .. } = &rosomaxa.phase else { unreachable!() };
    assert_eq!(maintenance.new_input_count, 1);

    let max_network_size = rosomaxa.config.max_network_size;
    let RosomaxaPhases::Exploration { maintenance, .. } = &mut rosomaxa.phase else { unreachable!() };
    maintenance.add_observations(max_network_size);
    rosomaxa.on_generation(&HeuristicStatistics { termination_estimate: 0.5, ..Default::default() });
    let RosomaxaPhases::Exploration { maintenance, .. } = &rosomaxa.phase else { unreachable!() };
    assert_eq!(maintenance.new_input_count, 0);
}

#[test]
fn can_adapt_smoothing_observation_window() {
    let config = RosomaxaConfig { max_network_size: 600, node_size: 2, ..RosomaxaConfig::new_with_defaults(4) };
    let mut maintenance = NetworkMaintenance::new(&config);

    maintenance.add_observations(99);
    assert_eq!(maintenance.next_action(100), None);
    maintenance.add_observations(1);
    assert_eq!(maintenance.next_action(100), Some(NetworkMaintenanceAction::CheckDistortion));

    maintenance.on_smoothing();
    assert_eq!(maintenance.new_input_count, 0);
    maintenance.add_observations(100);
    assert_eq!(maintenance.next_action(100), Some(NetworkMaintenanceAction::RefreshNormalization));
    maintenance.add_observations(100);
    assert_eq!(maintenance.next_action(100), Some(NetworkMaintenanceAction::CheckDistortion));

    maintenance.on_smoothing();
    maintenance.on_smoothing();
    maintenance.on_smoothing();
    for _ in 0..7 {
        maintenance.add_observations(100);
        assert_eq!(maintenance.next_action(100), Some(NetworkMaintenanceAction::RefreshNormalization));
    }
    maintenance.add_observations(100);
    assert_eq!(maintenance.next_action(100), Some(NetworkMaintenanceAction::CheckDistortion));

    maintenance.on_stable_observation();
    for _ in 0..3 {
        maintenance.add_observations(100);
        assert_eq!(maintenance.next_action(100), Some(NetworkMaintenanceAction::RefreshNormalization));
    }
    maintenance.add_observations(100);
    assert_eq!(maintenance.next_action(100), Some(NetworkMaintenanceAction::CheckDistortion));
}

#[test]
fn can_bound_smoothing_observation_window_for_different_node_sizes() {
    assert_eq!(get_max_smoothing_observation_multiplier(1), 4);
    assert_eq!(get_max_smoothing_observation_multiplier(2), 8);
    assert_eq!(get_max_smoothing_observation_multiplier(4), 16);
    assert_eq!(get_max_smoothing_observation_multiplier(10), 16);
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
