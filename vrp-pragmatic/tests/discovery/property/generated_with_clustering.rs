use crate::format::problem::*;
use crate::generator::*;
use crate::helpers::solve_with_metaheuristic_and_iterations;
use proptest::prelude::*;

fn job_prototype() -> impl Strategy<Value = Job> {
    prop_oneof![
        delivery_job_prototype(
            job_task_prototype(default_job_place_prototype(), generate_simple_demand(1..5), generate_no_order(),),
            generate_no_jobs_skills(),
            generate_no_jobs_value(),
            generate_no_jobs_group(),
        ),
        pickup_job_prototype(
            job_task_prototype(default_job_place_prototype(), generate_simple_demand(1..5), generate_no_order(),),
            generate_no_jobs_skills(),
            generate_no_jobs_value(),
            generate_no_jobs_group(),
        )
    ]
}

prop_compose! {
    fn get_problem_with_vicinity(radius: f64)
    (
     radius_fraction in 1..100,
     moving_duration in 30..1800,
     plan in generate_plan(generate_jobs(job_prototype(), 1..512)),
     fleet in generate_fleet(
        generate_vehicles(
             generate_vehicle(
                2..4,
                Just(VehicleProfile { matrix: "car".to_string(), scale: None }),
                generate_simple_capacity(5..20),
                default_costs_prototype(),
                generate_no_vehicle_skills(),
                generate_no_limits(),
                default_vehicle_shifts(),
            ), 1..4),
        default_matrix_profiles())
    ) -> Problem {
        let moving_duration = moving_duration as f64;
        let moving_distance = radius * (radius_fraction as f64 / 1000.);

        Problem {
            plan: Plan {
                clustering: Some(Clustering::Vicinity {
                    profile: VehicleProfile { matrix: "car".to_string(), scale: None },
                    threshold: VicinityThresholdPolicy {
                        moving_duration,
                        moving_distance,
                        min_shared_time: None,
                        smallest_time_window: None,
                        max_jobs_per_cluster: None,
                    },
                    visiting: VicinityVisitPolicy::Continue,
                    serving: VicinityServingPolicy::Original,
                    filtering: None,
                }),
                ..plan
            },
            fleet,
            objectives: None,
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    #[ignore]
    fn can_solve_problem_with_vicinity(problem in get_problem_with_vicinity(get_default_bounding_box_radius())) {
        let matrices = create_approx_matrices(&problem);
        solve_with_metaheuristic_and_iterations(problem, Some(matrices), 1);
    }
}
