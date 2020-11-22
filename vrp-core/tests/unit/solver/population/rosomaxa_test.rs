use super::SelectionPhase::{Exploitation, Exploration, Initial};
use super::*;
use crate::helpers::models::domain::*;

fn create_rosomaxa() -> Rosomaxa {
    let mut config = RosomaxaConfig::default();
    config.rebalance_memory = 10;
    config.selection_size = 4;

    Rosomaxa::new(create_empty_problem(), test_random(), config).unwrap()
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
    let mut rosomaxa = create_rosomaxa();

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
fn can_optimize_network() {
    let mut rosomaxa = create_rosomaxa();
    (0..10).for_each(|idx| {
        rosomaxa.add_all(vec![create_empty_insertion_context()]);
        rosomaxa.update_phase(&create_statistics(0., idx))
    });
    assert_eq!(get_network(&rosomaxa).get_nodes().count(), 4);

    rosomaxa.add(create_empty_insertion_context());
    rosomaxa.update_phase(&create_statistics(0., 10));

    assert_eq!(get_network(&rosomaxa).get_nodes().count(), 1);
}
