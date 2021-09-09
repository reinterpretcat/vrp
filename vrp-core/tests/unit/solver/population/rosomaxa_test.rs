use super::SelectionPhase::{Exploitation, Exploration, Initial};
use super::*;
use crate::helpers::models::domain::*;
use crate::utils::Timer;

fn create_rosomaxa(rebalance_memory: usize) -> Rosomaxa {
    let problem = create_empty_problem();
    let mut config = RosomaxaConfig::new_with_defaults(4);
    config.rebalance_memory = rebalance_memory;

    Rosomaxa::new(problem.objective.clone(), Arc::new(Environment::default()), config).unwrap()
}

fn create_statistics(termination_estimate: f64, generation: usize) -> Statistics {
    let mut statistics = Statistics::default();
    statistics.termination_estimate = termination_estimate;
    statistics.generation = generation;
    statistics.improvement_1000_ratio = 0.5;

    statistics
}

fn get_network(rosomaxa: &Rosomaxa) -> &IndividualNetwork {
    match &rosomaxa.phase {
        RosomaxaPhases::Exploration { network, .. } => network,
        _ => unreachable!(),
    }
}

#[test]
fn can_switch_phases() {
    let mut rosomaxa = create_rosomaxa(10);

    (0..4).for_each(|_| {
        assert_eq!(rosomaxa.selection_phase(), Initial);
        rosomaxa.add_all(vec![create_empty_insertion_context()]);
        rosomaxa.update_phase(&create_statistics(0., 0))
    });

    rosomaxa.add(create_empty_insertion_context());
    assert_eq!(rosomaxa.selection_phase(), Exploration);

    for (idx, (termination_estimate, phase)) in (&[(0.7, Exploration), (0.9, Exploitation)]).iter().enumerate() {
        rosomaxa.update_phase(&create_statistics(*termination_estimate, idx));
        assert_eq!(rosomaxa.selection_phase(), *phase);
    }
}

#[test]
fn can_select_individuals_in_different_phases() {
    let mut rosomaxa = create_rosomaxa(10);
    (0..10).for_each(|idx| {
        rosomaxa.add_all(vec![create_empty_insertion_context()]);
        rosomaxa.update_phase(&create_statistics(0.75, idx))
    });

    let individuals = rosomaxa.select();
    assert_eq!(individuals.count(), 4);
    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploration);

    rosomaxa.update_phase(&create_statistics(0.95, 10));
    let individuals = rosomaxa.select();
    assert_eq!(individuals.count(), 4);
    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploitation);
}

#[test]
fn can_optimize_network() {
    let termination_estimate = 0.75;
    let mut rosomaxa = create_rosomaxa(2);
    (0..10).for_each(|idx| {
        rosomaxa.add_all(vec![create_simple_insertion_ctx(idx as f64, idx)]);
        rosomaxa.update_phase(&create_statistics(termination_estimate, idx))
    });

    rosomaxa.add(create_empty_insertion_context());
    rosomaxa.update_phase(&create_statistics(termination_estimate, 10));

    assert_eq!(get_network(&rosomaxa).get_nodes().count(), 1);
}

#[test]
fn can_format_network() {
    let mut rosomaxa = create_rosomaxa(4);
    rosomaxa.add_all(vec![create_empty_insertion_context()]);

    let str = format!("{}", rosomaxa);

    assert_eq!(str, "[[0.0000000,0.0000000,0.0000000],]");
}

#[test]
fn can_handle_empty_population() {
    let mut rosomaxa = create_rosomaxa(10);

    for (phase, estimate) in vec![(Initial, None), (Initial, Some(0.7)), (Initial, Some(0.95))] {
        if let Some(estimate) = estimate {
            rosomaxa.update_phase(&create_statistics(estimate, 10));
        }

        assert!(rosomaxa.select().next().is_none());
        assert_eq!(rosomaxa.selection_phase(), phase)
    }
}

#[test]
fn can_get_population_shuffle_amount() {
    let high_improvement = |termination_estimate: f64| Statistics {
        generation: 1,
        time: Timer::start(),
        speed: RefinementSpeed::Moderate,
        improvement_all_ratio: 0.99,
        improvement_1000_ratio: 0.99,
        termination_estimate,
    };
    assert_eq!(Rosomaxa::get_shuffle_amount(&high_improvement(0.), 100), 50);
    assert_eq!(Rosomaxa::get_shuffle_amount(&high_improvement(0.20), 100), 48);
    assert_eq!(Rosomaxa::get_shuffle_amount(&high_improvement(0.25), 100), 46);
    assert_eq!(Rosomaxa::get_shuffle_amount(&high_improvement(0.5), 100), 25);
    assert_eq!(Rosomaxa::get_shuffle_amount(&high_improvement(0.55), 100), 19);
    assert_eq!(Rosomaxa::get_shuffle_amount(&high_improvement(0.65), 100), 10);
    assert_eq!(Rosomaxa::get_shuffle_amount(&high_improvement(0.75), 100), 10);
    assert_eq!(Rosomaxa::get_shuffle_amount(&high_improvement(1.), 100), 10);

    let some_improvement = |ratio: f64, termination_estimate: f64| Statistics {
        generation: 1,
        time: Timer::start(),
        speed: RefinementSpeed::Moderate,
        improvement_all_ratio: ratio,
        improvement_1000_ratio: ratio,
        termination_estimate,
    };
    assert_eq!(Rosomaxa::get_shuffle_amount(&some_improvement(0.3, 0.55), 100), 50);
    assert_eq!(Rosomaxa::get_shuffle_amount(&some_improvement(0.1, 0.55), 100), 100);
}
