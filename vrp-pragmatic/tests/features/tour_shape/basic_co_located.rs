use crate::format::problem::Objective::*;
use crate::format::problem::*;
use crate::format::solution::Solution;
use crate::helpers::*;

fn create_problem(jobs: Vec<Job>, capacity: i32, pairs: &[(&str, &str)]) -> Problem {
    Problem {
        plan: Plan { jobs, ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["my_vehicle_1".to_string(), "my_vehicle_2".to_string()],
                shifts: vec![create_default_open_vehicle_shift()],
                ..create_vehicle_with_capacity("my_vehicle", vec![capacity])
            }],
            ..create_default_fleet()
        },
        objectives: Some(vec![
            MinimizeUnassigned { breaks: None },
            MinimizeSplitCoLocated { pairs: pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect() },
            MinimizeCost,
        ]),
        ..create_empty_problem()
    }
}

fn get_vehicle_of(solution: &Solution, job_id: &str) -> Option<String> {
    solution
        .tours
        .iter()
        .find(|tour| get_ids_from_tour(tour).into_iter().flatten().any(|id| id == job_id))
        .map(|tour| tour.vehicle_id.clone())
}

#[test]
fn can_keep_co_located_pair_in_one_tour_against_cost() {
    // Cost alone serves each side of the depot with its own vehicle, splitting job-1 from job1.
    let problem = create_problem(
        vec![
            create_delivery_job("job-2", (-2., 0.)),
            create_delivery_job("job-1", (-1., 0.)),
            create_delivery_job("job1", (1., 0.)),
            create_delivery_job("job2", (2., 0.)),
        ],
        2,
        &[("job-1", "job1")],
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), 500);

    assert!(solution.unassigned.is_none());
    assert_eq!(get_vehicle_of(&solution, "job-1"), get_vehicle_of(&solution, "job1"));
}

#[test]
fn can_split_co_located_pair_when_that_places_a_job() {
    // One job per vehicle: keeping the pair together would leave one of them unassigned.
    let problem = create_problem(
        vec![create_delivery_job("job1", (1., 0.)), create_delivery_job("job2", (1., 0.))],
        1,
        &[("job1", "job2")],
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), 500);

    assert!(solution.unassigned.is_none());
    assert_ne!(get_vehicle_of(&solution, "job1"), get_vehicle_of(&solution, "job2"));
}
