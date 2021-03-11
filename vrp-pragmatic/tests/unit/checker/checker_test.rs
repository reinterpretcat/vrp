use super::*;
use crate::helpers::*;

#[test]
fn can_remove_duplicates_in_error_list() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", vec![2., 0.])], relations: None },
        fleet: Fleet { vehicles: vec![create_default_vehicle_type()], profiles: create_default_profiles() },
        ..create_empty_problem()
    };
    let solution = Solution {
        tours: vec![Tour {
            vehicle_id: "my_vehicle_11".to_string(),
            type_id: "my_vehicle".to_string(),
            stops: vec![
                create_stop_with_activity(
                    "departure",
                    "departure",
                    (0., 0.),
                    1,
                    ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    0,
                ),
                create_stop_with_activity(
                    "job1",
                    "delivery",
                    (2., 0.),
                    0,
                    ("1970-01-01T00:00:02Z", "1970-01-01T00:00:03Z"),
                    2,
                ),
                create_stop_with_activity(
                    "arrival",
                    "arrival",
                    (0., 0.),
                    0,
                    ("1970-01-01T00:00:05Z", "1970-01-01T00:00:05Z"),
                    4,
                ),
            ],
            ..create_empty_tour()
        }],
        ..create_empty_solution()
    };
    let core_problem = Arc::new(problem.clone().read_pragmatic().unwrap());

    let result = CheckerContext::new(core_problem, problem, None, solution).check();

    assert_eq!(
        result,
        Err(vec![
            "cannot find vehicle with id 'my_vehicle_11'".to_owned(),
            "used vehicle with unknown id: 'my_vehicle_11'".to_owned()
        ])
    );
}
