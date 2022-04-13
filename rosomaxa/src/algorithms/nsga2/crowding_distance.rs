#[cfg(test)]
#[path = "../../../tests/unit/algorithms/nsga2/crowding_distance_test.rs"]
mod crowding_distance_test;

use crate::algorithms::nsga2::non_dominated_sort::Front;
use crate::algorithms::nsga2::*;

pub struct AssignedCrowdingDistance<'a, S>
where
    S: 'a,
{
    pub index: usize,
    pub solution: &'a S,
    pub rank: usize,
    pub crowding_distance: f64,
}

pub struct ObjectiveStat {
    pub spread: f64,
}

/// Assigns a crowding distance to each solution in `front`.
pub fn assign_crowding_distance<'a, S>(
    front: &Front<'a, S>,
    multi_objective: &impl MultiObjective<Solution = S>,
) -> (Vec<AssignedCrowdingDistance<'a, S>>, Vec<ObjectiveStat>) {
    let mut a: Vec<_> = front
        .iter()
        .map(|(solution, index)| AssignedCrowdingDistance {
            index,
            solution,
            rank: front.rank(),
            crowding_distance: 0.0,
        })
        .collect();

    let objective_count = multi_objective.objectives().count();

    let objective_stat: Vec<_> = multi_objective
        .objectives()
        .map(|objective| {
            // first, sort according to objective
            a.sort_by(|a, b| objective.total_order(a.solution, b.solution));

            // assign infinite crowding distance to the extremes
            {
                a.first_mut().unwrap().crowding_distance = f64::INFINITY;
                a.last_mut().unwrap().crowding_distance = f64::INFINITY;
            }

            // the distance between the "best" and "worst" solution according to "objective"
            let spread = objective.distance(a.first().unwrap().solution, a.last().unwrap().solution).abs();
            debug_assert!(spread >= 0.0);

            if spread > 0.0 {
                let norm = 1.0 / (spread * (objective_count as f64));
                debug_assert!(norm > 0.0);

                for i in 1..a.len() - 1 {
                    debug_assert!(i >= 1 && i + 1 < a.len());

                    let distance = objective.distance(a[i + 1].solution, a[i - 1].solution).abs();
                    debug_assert!(distance >= 0.0);
                    a[i].crowding_distance += distance * norm;
                }
            }

            ObjectiveStat { spread }
        })
        .collect();

    (a, objective_stat)
}
