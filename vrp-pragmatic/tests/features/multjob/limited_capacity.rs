use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_handle_limited_capacity() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_multi_job(
                    "multi_1",
                    vec![((1., 0.), 1., vec![1]), ((2., 0.), 1., vec![1])],
                    vec![((10., 0.), 1., vec![2])],
                ),
                create_multi_job(
                    "multi_2",
                    vec![((3., 0.), 1., vec![1]), ((4., 0.), 1., vec![1])],
                    vec![((11., 0.), 1., vec![2])],
                ),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_capacity("my_vehicle", vec![2])],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution.statistic,
        Statistic {
            cost: 88.,
            distance: 36,
            duration: 42,
            times: Timing { driving: 36, serving: 6, waiting: 0, break_time: 0 },
        }
    );
    assert!(solution.unassigned.is_none());
}
