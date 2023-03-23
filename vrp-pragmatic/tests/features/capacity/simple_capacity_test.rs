use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_schedule_pickup_at_tour_end() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_demand("job1", (1., 0.), vec![1]),
                create_delivery_job_with_demand("job2", (2., 0.), vec![1]),
                create_pickup_job_with_demand("job3", (3., 0.), vec![2]),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType { capacity: vec![2], ..create_default_vehicle_type() }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_cheapest_insertion(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none())
}
