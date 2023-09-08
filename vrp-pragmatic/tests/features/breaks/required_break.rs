use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

fn create_shift_start() -> ShiftStart {
    ShiftStart { earliest: format_time(0.), latest: Some(format_time(0.)), location: (0., 0.).to_loc() }
}

fn create_problem(jobs: Vec<Job>, vehicle_break: VehicleBreak, is_open: bool) -> Problem {
    let vehicle_shift = if is_open { create_default_open_vehicle_shift() } else { create_default_vehicle_shift() };
    Problem {
        plan: Plan { jobs, ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    start: create_shift_start(),
                    breaks: Some(vec![vehicle_break]),
                    ..vehicle_shift
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    }
}

#[test]
fn can_assign_break_during_travel() {
    let is_open = false;
    let problem = create_problem(
        vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (10., 0.))],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(7.), latest: format_time(7.) },
            duration: 2.,
        },
        is_open,
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![2])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(5., 6.)
                            .load(vec![1])
                            .distance(5)
                            .build_single("job1", "delivery"),
                        StopBuilder::new_transit().schedule_stamp(7., 9.).load(vec![1]).build_single("break", "break"),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(13., 14.)
                            .load(vec![0])
                            .distance(10)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(24., 24.)
                            .load(vec![0])
                            .distance(20)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(20).serving(2).break_time(2).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_assign_break_during_activity() {
    let is_open = false;
    let problem = create_problem(
        vec![create_delivery_job_with_duration("job1", (5., 0.), 3.)],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(7.), latest: format_time(7.) },
            duration: 2.,
        },
        is_open,
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![1])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(5., 10.)
                            .load(vec![0])
                            .distance(5)
                            .activity(
                                ActivityBuilder::delivery()
                                    .job_id("job1")
                                    .coordinate((5., 0.))
                                    .time_stamp(5., 10.)
                                    .build()
                            )
                            .activity(ActivityBuilder::break_type().time_stamp(7., 9.).build())
                            .build(),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(15., 15.)
                            .load(vec![0])
                            .distance(10)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(10).serving(3).break_time(2).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_handle_required_break_when_its_start_falls_at_activity_end() {
    let is_open = true;
    let problem = create_problem(
        vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (10., 0.))],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(6.), latest: format_time(6.) },
            duration: 2.,
        },
        is_open,
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![2])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(5., 6.)
                            .load(vec![1])
                            .distance(5)
                            .build_single("job1", "delivery"),
                        StopBuilder::new_transit().schedule_stamp(6., 8.).load(vec![1]).build_single("break", "break"),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(13., 14.)
                            .load(vec![0])
                            .distance(10)
                            .build_single("job2", "delivery"),
                    ])
                    .statistic(StatisticBuilder::default().driving(10).serving(2).break_time(2).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_skip_break_if_it_is_after_start_before_end_range() {
    let is_open = true;
    let problem = create_problem(
        vec![create_delivery_job("job1", (5., 0.))],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(5.), latest: format_time(7.) },
            duration: 2.,
        },
        is_open,
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(get_ids_from_tour(&solution.tours[0]).iter().flatten().all(|id| id != "break"));
}

// TODO check exact and offset use cases
#[test]
fn can_reschedule_break_early_from_transport_to_activity() {
    let is_open = true;
    let problem = create_problem(
        vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (10., 0.))],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(4.), latest: format_time(7.) },
            duration: 2.,
        },
        is_open,
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![2])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(5., 8.)
                            .load(vec![1])
                            .distance(5)
                            .activity(
                                ActivityBuilder::delivery()
                                    .job_id("job1")
                                    .coordinate((5., 0.))
                                    .time_stamp(5., 6.)
                                    .build()
                            )
                            .activity(ActivityBuilder::break_type().time_stamp(6., 8.).build())
                            .build(),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(13., 14.)
                            .load(vec![0])
                            .distance(10)
                            .build_single("job2", "delivery"),
                    ])
                    .statistic(StatisticBuilder::default().driving(10).serving(2).break_time(2).build())
                    .build()
            )
            .build()
    );
}
