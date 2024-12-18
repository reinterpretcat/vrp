use super::*;
use crate::example::*;
use crate::helpers::example::create_example_objective;

fn create_rosomaxa(
    rebalance_memory: usize,
) -> (Arc<VectorObjective>, Rosomaxa<VectorRosomaxaContext, VectorObjective, VectorSolution>) {
    let mut config = RosomaxaConfig::new_with_defaults(4);
    config.rebalance_memory = rebalance_memory;

    let objective = create_example_objective();
    let population =
        Rosomaxa::new(VectorRosomaxaContext, objective.clone(), Arc::new(Environment::default()), config).unwrap();

    (objective, population)
}

fn create_statistics(termination_estimate: Float, generation: usize) -> HeuristicStatistics {
    HeuristicStatistics {
        termination_estimate,
        generation,
        improvement_1000_ratio: 0.5,
        ..HeuristicStatistics::default()
    }
}

fn get_network(
    rosomaxa: &Rosomaxa<VectorRosomaxaContext, VectorObjective, VectorSolution>,
) -> &IndividualNetwork<VectorRosomaxaContext, VectorObjective, VectorSolution> {
    match &rosomaxa.phase {
        RosomaxaPhases::Exploration { network, .. } => network,
        _ => unreachable!(),
    }
}

#[test]
fn can_switch_phases() {
    let (objective, mut rosomaxa) = create_rosomaxa(10);

    (0..4).for_each(|_| {
        assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Initial);
        rosomaxa.add_all(vec![VectorSolution::new_with_objective(vec![-1., -1.], objective.as_ref())]);
        rosomaxa.update_phase(&create_statistics(0., 0))
    });

    rosomaxa.add(VectorSolution::new_with_objective(vec![-1., -1.], objective.as_ref()));
    assert_eq!(rosomaxa.selection_phase(), SelectionPhase::Exploration);

    for (idx, (termination_estimate, phase)) in
        ([(0.7, SelectionPhase::Exploration), (0.9, SelectionPhase::Exploitation)]).iter().enumerate()
    {
        rosomaxa.update_phase(&create_statistics(*termination_estimate, idx));
        assert_eq!(rosomaxa.selection_phase(), *phase);
    }
}

#[test]
fn can_select_individuals_in_different_phases() {
    let (objective, mut rosomaxa) = create_rosomaxa(10);
    (0..10).for_each(|idx| {
        rosomaxa.add_all(vec![VectorSolution::new_with_objective(vec![-1., -1.], objective.as_ref())]);
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
    let (objective, mut rosomaxa) = create_rosomaxa(2);
    (0..10).for_each(|idx| {
        let value = idx as Float - 5.;
        rosomaxa.add_all(vec![VectorSolution::new_with_objective(vec![value, value], objective.as_ref())]);
        rosomaxa.update_phase(&create_statistics(termination_estimate, idx))
    });

    rosomaxa.add(VectorSolution::new_with_objective(vec![0.5, 0.5], objective.as_ref()));
    rosomaxa.update_phase(&create_statistics(termination_estimate, 10));

    assert!(get_network(&rosomaxa).get_nodes().next().is_some());
}

#[test]
fn can_handle_empty_population() {
    let (_, mut rosomaxa) = create_rosomaxa(10);

    for (phase, estimate) in
        [(SelectionPhase::Initial, None), (SelectionPhase::Initial, Some(0.7)), (SelectionPhase::Initial, Some(0.95))]
    {
        if let Some(estimate) = estimate {
            rosomaxa.update_phase(&create_statistics(estimate, 10));
        }

        assert!(rosomaxa.select().next().is_none());
        assert_eq!(rosomaxa.selection_phase(), phase)
    }
}
