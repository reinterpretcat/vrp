use crate::format::problem::Objective::*;
use crate::format::problem::*;
use crate::format::solution::Tour;
use crate::helpers::*;
use std::collections::HashMap;

fn get_activities_count(tour: &Tour) -> usize {
    tour.stops
        .iter()
        .map(|stop| stop.activities().iter().filter(|activity| activity.activity_type == "delivery").count())
        .sum()
}

#[test]
fn can_balance_production_value() {
    // Deliberately unequal per-job values: group1 (4 jobs @ (1,0), near my_vehicle1's own depot)
    // are worth 10 each (total 40); group2 (2 jobs @ (2,0), near my_vehicle2's own depot) has one
    // job worth 30 and one worth 10 (total 40). The natural, cheapest (no-crossover) assignment --
    // my_vehicle1 serves all 4 group1 jobs, my_vehicle2 serves both group2 jobs -- is a 4/2 job
    // split that already balances total VALUE (40/40). A count-based implementation (e.g. one that
    // mistakenly reads job count instead of the productionValue dimension) would instead see this
    // as an unbalanced 4/2 *count* split and push towards 3/3 (which requires costlier crossover),
    // so asserting the 4/2 split here discriminates a correct value read from a count-based bug.
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_production_value("job1.0", (1., 0.), 10.),
                create_delivery_job_with_production_value("job1.1", (1., 0.), 10.),
                create_delivery_job_with_production_value("job1.2", (1., 0.), 10.),
                create_delivery_job_with_production_value("job1.3", (1., 0.), 10.),
                create_delivery_job_with_production_value("job2.0", (2., 0.), 30.),
                create_delivery_job_with_production_value("job2.1", (2., 0.), 10.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![
                VehicleType {
                    vehicle_ids: vec!["my_vehicle1".to_string()],
                    shifts: vec![create_default_open_vehicle_shift()],
                    capacity: vec![4],
                    ..create_default_vehicle_type()
                },
                VehicleType {
                    type_id: "my_vehicle2".to_string(),
                    vehicle_ids: vec!["my_vehicle2".to_string()],
                    shifts: vec![create_default_vehicle_shift_with_locations((3., 0.), (3., 0.))],
                    capacity: vec![4],
                    ..create_default_vehicle_type()
                },
            ],
            ..create_default_fleet()
        },
        objectives: Some(vec![MinimizeUnassigned { breaks: None }, BalanceProductionValue, MinimizeCost]),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    let totals = solution.tours.iter().fold(HashMap::<String, usize>::new(), |mut acc, tour| {
        *acc.entry(tour.vehicle_id.clone()).or_insert(0) += get_activities_count(tour);
        acc
    });

    assert_eq!(totals.get("my_vehicle1").copied().unwrap_or(0), 4);
    assert_eq!(totals.get("my_vehicle2").copied().unwrap_or(0), 2);
}
