use super::SelectionPhase::{Exploitation, Exploration, Initial};
use super::*;
use crate::helpers::models::domain::*;

fn create_rosomaxa(rebalance_memory: usize) -> Rosomaxa {
    let mut config = RosomaxaConfig::new_with_defaults(4);
    config.rebalance_memory = rebalance_memory;

    Rosomaxa::new(create_empty_problem(), Arc::new(Environment::default()), config).unwrap()
}

fn create_statistics(termination_estimate: f64, generation: usize) -> Statistics {
    let mut statistics = Statistics::default();
    statistics.termination_estimate = termination_estimate;
    statistics.generation = generation;
    statistics.improvement_1000_ratio = 0.5;

    statistics
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
