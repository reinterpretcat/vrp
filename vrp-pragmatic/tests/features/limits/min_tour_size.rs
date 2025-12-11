use crate::format::problem::*;
use crate::format::solution::Solution;
use crate::helpers::*;

fn create_objectives_with_min_tour_size() -> Option<Vec<Objective>> {
    Some(vec![
        Objective::MinimizeUnassigned { breaks: None },
        Objective::MinimizeTourSizeViolation,
        Objective::MinimizeTours,
        Objective::MinimizeCost,
    ])
}

fn create_maximize_tours_objectives() -> Option<Vec<Objective>> {
    // Maximize tours to force creating more routes (opposite of minimize)
    Some(vec![
        Objective::MinimizeUnassigned { breaks: None },
        Objective::MaximizeTours,
        Objective::MinimizeCost,
    ])
}

fn create_maximize_tours_with_min_tour_size_objectives() -> Option<Vec<Objective>> {
    // Maximize tours BUT also minimize tour size violations
    // The MinimizeTourSizeViolation should counteract MaximizeTours when routes become too small
    Some(vec![
        Objective::MinimizeUnassigned { breaks: None },
        Objective::MinimizeTourSizeViolation,
        Objective::MaximizeTours,
        Objective::MinimizeCost,
    ])
}

fn count_job_activities(solution: &Solution) -> Vec<usize> {
    solution
        .tours
        .iter()
        .map(|tour| {
            tour.stops
                .iter()
                .flat_map(|stop| stop.activities())
                .filter(|a| a.activity_type != "departure" && a.activity_type != "arrival")
                .count()
        })
        .collect()
}

fn has_underfilled_routes(activity_counts: &[usize], min_size: usize) -> bool {
    activity_counts.iter().any(|&count| count > 0 && count < min_size)
}

/// This test verifies that the MinimizeTourSizeViolation objective actually changes behavior.
/// We use MaximizeTours to force creating multiple routes, then show that MinimizeTourSizeViolation
/// prevents routes from having fewer than min_tour_size activities.
#[test]
fn can_verify_objective_changes_behavior() {
    // Problem: 4 jobs, 4 vehicles with high capacity, min_tour_size=2
    // Default behavior (no objectives defined): minimizes unassigned, tours, then cost
    // With MaximizeTours + MinimizeTourSizeViolation: tries to create more tours but penalizes underfilled ones
    let create_problem = |objectives: Option<Vec<Objective>>, with_min_tour_size: bool| Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_demand("job1", (1., 0.), vec![1]),
                create_delivery_job_with_demand("job2", (2., 0.), vec![1]),
                create_delivery_job_with_demand("job3", (3., 0.), vec![1]),
                create_delivery_job_with_demand("job4", (4., 0.), vec![1]),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec![
                    "v1".to_string(),
                    "v2".to_string(),
                    "v3".to_string(),
                    "v4".to_string(),
                ],
                shifts: vec![create_default_open_vehicle_shift()],
                capacity: vec![10], // High capacity - not a limiting factor
                limits: if with_min_tour_size {
                    Some(VehicleLimits {
                        max_distance: None,
                        max_duration: None,
                        tour_size: None,
                        min_tour_size: Some(2),
                    })
                } else {
                    None
                },
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        objectives,
        ..create_empty_problem()
    };

    let matrix_without = create_matrix_from_problem(&create_problem(None, false));
    let matrix_with = create_matrix_from_problem(&create_problem(None, true));

    // Solve with MaximizeTours but WITHOUT min_tour_size limit on vehicles
    // This should create as many routes as possible (4 routes with 1 job each)
    let solution_without = solve_with_metaheuristic_and_iterations(
        create_problem(create_maximize_tours_objectives(), false),
        Some(vec![matrix_without]),
        200,
    );

    // Solve with MaximizeTours AND MinimizeTourSizeViolation (with min_tour_size on vehicles)
    // This should balance more tours vs the penalty for underfilled routes
    let solution_with = solve_with_metaheuristic_and_iterations(
        create_problem(create_maximize_tours_with_min_tour_size_objectives(), true),
        Some(vec![matrix_with]),
        200,
    );

    let counts_without = count_job_activities(&solution_without);
    let counts_with = count_job_activities(&solution_with);

    let underfilled_without = has_underfilled_routes(&counts_without, 2);
    let underfilled_with = has_underfilled_routes(&counts_with, 2);

    println!("MaximizeTours only: {} tours with activities {:?}, underfilled: {}", 
             solution_without.tours.len(), counts_without, underfilled_without);
    println!("MaximizeTours + MinTourSizeViolation: {} tours with activities {:?}, underfilled: {}", 
             solution_with.tours.len(), counts_with, underfilled_with);

    // The key assertion: Without the objective, MaximizeTours creates underfilled routes
    assert!(
        underfilled_without,
        "Without objective, MaximizeTours should create underfilled routes. Got: {:?}",
        counts_without
    );

    // With the objective, routes should NOT be underfilled
    assert!(
        !underfilled_with,
        "With objective, should have no underfilled routes. Got: {:?}",
        counts_with
    );

    // All jobs should be assigned in the solution with objective
    assert!(
        solution_with.unassigned.is_none() || solution_with.unassigned.as_ref().unwrap().is_empty(),
        "Solution with objective has unassigned jobs"
    );
}

#[test]
fn can_enforce_min_tour_size_by_consolidating_jobs() {
    // Problem: 4 jobs, 2 vehicles, min_tour_size=2
    // Expected: Solver should create routes with at least 2 jobs each,
    // rather than spreading jobs across more routes with fewer jobs each.
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                create_delivery_job("job3", (3., 0.)),
                create_delivery_job("job4", (4., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["v1".to_string(), "v2".to_string()],
                shifts: vec![create_default_open_vehicle_shift()],
                limits: Some(VehicleLimits {
                    max_distance: None,
                    max_duration: None,
                    tour_size: None,
                    min_tour_size: Some(2),
                }),
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        objectives: create_objectives_with_min_tour_size(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), 100);

    // Verify that all routes have at least 2 activities (excluding departure/arrival)
    for tour in &solution.tours {
        let job_activities: usize = tour
            .stops
            .iter()
            .flat_map(|stop| stop.activities())
            .filter(|a| a.activity_type != "departure" && a.activity_type != "arrival")
            .count();
        
        assert!(
            job_activities >= 2 || job_activities == 0,
            "Tour for vehicle {} has {} job activities, expected at least 2 (or 0 if empty)",
            tour.vehicle_id,
            job_activities
        );
    }

    // Verify all jobs are assigned
    assert!(
        solution.unassigned.is_none() || solution.unassigned.as_ref().unwrap().is_empty(),
        "Expected all jobs to be assigned, but found unassigned: {:?}",
        solution.unassigned
    );
}

#[test]
fn can_reject_solution_violating_min_tour_size() {
    // Problem: 3 jobs, 2 vehicles, min_tour_size=2
    // This creates a scenario where it's impossible to satisfy the constraint
    // (can't have 2 routes with 2+ jobs each when only 3 jobs exist)
    // The solver should put all jobs in one route or leave some unassigned
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                create_delivery_job("job3", (3., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["v1".to_string(), "v2".to_string()],
                shifts: vec![create_default_open_vehicle_shift()],
                limits: Some(VehicleLimits {
                    max_distance: None,
                    max_duration: None,
                    tour_size: None,
                    min_tour_size: Some(2),
                }),
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        objectives: create_objectives_with_min_tour_size(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), 100);

    // All routes must have either 0 or >= 2 activities
    for tour in &solution.tours {
        let job_activities: usize = tour
            .stops
            .iter()
            .flat_map(|stop| stop.activities())
            .filter(|a| a.activity_type != "departure" && a.activity_type != "arrival")
            .count();

        assert!(
            job_activities >= 2 || job_activities == 0,
            "Tour for vehicle {} has {} job activities, expected at least 2 (or 0 if empty)",
            tour.vehicle_id,
            job_activities
        );
    }
}

#[test]
fn can_handle_min_tour_size_with_single_vehicle() {
    // Problem: 3 jobs, 1 vehicle, min_tour_size=2
    // Expected: All jobs should be assigned to the single vehicle
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                create_delivery_job("job3", (3., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_open_vehicle_shift()],
                limits: Some(VehicleLimits {
                    max_distance: None,
                    max_duration: None,
                    tour_size: None,
                    min_tour_size: Some(2),
                }),
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        objectives: create_objectives_with_min_tour_size(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), 100);

    // Should have one tour with all 3 jobs
    assert_eq!(solution.tours.len(), 1, "Expected exactly one tour");
    
    let job_activities: usize = solution.tours[0]
        .stops
        .iter()
        .flat_map(|stop| stop.activities())
        .filter(|a| a.activity_type != "departure" && a.activity_type != "arrival")
        .count();
    
    assert_eq!(job_activities, 3, "Expected all 3 jobs in the tour");
}
