use crate::format::problem::Objective::*;
use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_assign_jobs_to_nearest_vehicle() {
    // Two vehicles: v1 at (0,0), v2 at (20,0).
    // Two clusters of jobs: near v1 at (1,0),(2,0),(3,0) and near v2 at (18,0),(19,0),(20,0).
    // With MinimizeVehicleDistance, jobs should be assigned to their nearest vehicle.
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                create_delivery_job("job3", (3., 0.)),
                create_delivery_job("job4", (18., 0.)),
                create_delivery_job("job5", (19., 0.)),
                create_delivery_job("job6", (20., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![
                VehicleType {
                    vehicle_ids: vec!["v1_1".to_string()],
                    shifts: vec![create_default_vehicle_shift_with_locations((0., 0.), (0., 0.))],
                    ..create_vehicle_with_capacity("v1", vec![10])
                },
                VehicleType {
                    vehicle_ids: vec!["v2_1".to_string()],
                    shifts: vec![create_default_vehicle_shift_with_locations((20., 0.), (20., 0.))],
                    ..create_vehicle_with_capacity("v2", vec![10])
                },
            ],
            ..create_default_fleet()
        },
        objectives: Some(vec![MinimizeUnassigned { breaks: None }, MinimizeVehicleDistance, MinimizeCost]),
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), 500);

    assert!(solution.unassigned.is_none());
    assert_eq!(solution.tours.len(), 2);

    for tour in &solution.tours {
        let job_ids: Vec<&str> = tour
            .stops
            .iter()
            .flat_map(|stop| stop.activities().iter())
            .filter(|a| a.activity_type == "delivery")
            .map(|a| a.job_id.as_str())
            .collect();

        if tour.vehicle_id == "v1_1" {
            for id in &job_ids {
                assert!(["job1", "job2", "job3"].contains(id), "v1 should serve nearby jobs, but got {id}");
            }
        } else if tour.vehicle_id == "v2_1" {
            for id in &job_ids {
                assert!(["job4", "job5", "job6"].contains(id), "v2 should serve nearby jobs, but got {id}");
            }
        }
    }
}

/// Computes the total "excess distance" for a solution: for each job, how much farther
/// is the assigned vehicle compared to the nearest vehicle.
fn compute_excess_distance(solution: &crate::format::solution::Solution, vehicle_starts: &[(&str, (f64, f64))]) -> f64 {
    let dist = |a: (f64, f64), b: (f64, f64)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();

    let mut total_excess = 0.0;
    let mut job_count = 0;

    for tour in &solution.tours {
        let vehicle_start =
            vehicle_starts.iter().find(|(vid, _)| *vid == tour.vehicle_id).map(|(_, loc)| *loc).unwrap();

        for stop in &tour.stops {
            let Some(stop_loc) = stop.location().map(|l| l.to_lat_lng()) else { continue };
            for activity in stop.activities() {
                if activity.activity_type == "departure" || activity.activity_type == "arrival" {
                    continue;
                }

                let job_loc = activity.location.as_ref().map(|l| l.to_lat_lng()).unwrap_or(stop_loc);

                let dist_assigned = dist(job_loc, vehicle_start);
                let dist_nearest = vehicle_starts.iter().map(|(_, vloc)| dist(job_loc, *vloc)).fold(f64::MAX, f64::min);
                total_excess += (dist_assigned - dist_nearest).max(0.0);
                job_count += 1;
            }
        }
    }

    if job_count > 0 { total_excess / job_count as f64 } else { 0.0 }
}

#[test]
fn can_reduce_vehicle_distance_with_many_jobs() {
    // 5 vehicles spread along X-axis at (0,0), (20,0), (40,0), (60,0), (80,0)
    // 50 jobs in clusters of 10 around each vehicle position
    // Compare: with vs without MinimizeVehicleDistance
    let vehicle_positions: Vec<(f64, f64)> = vec![(0., 0.), (20., 0.), (40., 0.), (60., 0.), (80., 0.)];

    let mut jobs = Vec::new();
    for (cluster_idx, &(vx, vy)) in vehicle_positions.iter().enumerate() {
        for j in 0..10 {
            let offset_x = (j as f64 - 4.5) * 1.5; // spread jobs +-6.75 around vehicle
            let offset_y = (j as f64 % 3.0 - 1.0) * 2.0; // slight Y variation
            let job_id = format!("job_{}_{}", cluster_idx, j);
            jobs.push(create_delivery_job(&job_id, (vx + offset_x, vy + offset_y)));
        }
    }

    let vehicles: Vec<VehicleType> = vehicle_positions
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| VehicleType {
            vehicle_ids: vec![format!("v{}_1", i)],
            shifts: vec![create_default_vehicle_shift_with_locations((x, y), (x, y))],
            ..create_vehicle_with_capacity(&format!("v{i}"), vec![20])
        })
        .collect();

    let vehicle_starts: Vec<(&str, (f64, f64))> =
        vec![("v0_1", (0., 0.)), ("v1_1", (20., 0.)), ("v2_1", (40., 0.)), ("v3_1", (60., 0.)), ("v4_1", (80., 0.))];

    // --- Solve WITHOUT MinimizeVehicleDistance ---
    let problem_without = Problem {
        plan: Plan { jobs: jobs.clone(), ..create_empty_plan() },
        fleet: Fleet { vehicles: vehicles.clone(), ..create_default_fleet() },
        objectives: Some(vec![MinimizeUnassigned { breaks: None }, MinimizeTours, MinimizeCost]),
    };
    let matrix_without = create_matrix_from_problem(&problem_without);
    let solution_without = solve_with_metaheuristic_and_iterations(problem_without, Some(vec![matrix_without]), 500);

    // --- Solve WITH MinimizeVehicleDistance ---
    let problem_with = Problem {
        plan: Plan { jobs, ..create_empty_plan() },
        fleet: Fleet { vehicles, ..create_default_fleet() },
        objectives: Some(vec![MinimizeUnassigned { breaks: None }, MinimizeVehicleDistance, MinimizeCost]),
    };
    let matrix_with = create_matrix_from_problem(&problem_with);
    let solution_with = solve_with_metaheuristic_and_iterations(problem_with, Some(vec![matrix_with]), 500);

    // Both should assign all jobs
    assert!(solution_without.unassigned.is_none(), "without: has unassigned jobs");
    assert!(solution_with.unassigned.is_none(), "with: has unassigned jobs");

    let excess_without = compute_excess_distance(&solution_without, &vehicle_starts);
    let excess_with = compute_excess_distance(&solution_with, &vehicle_starts);

    eprintln!("=== MinimizeVehicleDistance effectiveness (50 jobs, 5 vehicles) ===");
    eprintln!("  Avg excess distance WITHOUT objective: {excess_without:.2}");
    eprintln!("  Avg excess distance WITH    objective: {excess_with:.2}");
    eprintln!("  Improvement: {:.1}%", (1.0 - excess_with / excess_without.max(0.001)) * 100.0);

    // The objective should meaningfully reduce excess distance
    assert!(
        excess_with <= excess_without,
        "MinimizeVehicleDistance should not increase excess distance: with={excess_with:.2}, without={excess_without:.2}"
    );
}
