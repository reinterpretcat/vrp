use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_use_vehicle_with_pickups_and_deliveries() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("d1", (1., 0.)),
                create_delivery_job("d2", (4., 0.)),
                create_delivery_job("d3", (10., 0.)),
                create_pickup_job("p1", (2., 0.)),
                create_pickup_job("p2", (5., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(100), location: (6., 0.).to_loc() }),
                    breaks: None,
                    reloads: Some(vec![VehicleReload {
                        location: (3., 0.).to_loc(),
                        duration: 2,
                        ..create_default_reload()
                    }]),
                    recharges: None,
                }],
                capacity: vec![1],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), 2000);

    assert_eq!(
        solution.tours,
        vec![TourBuilder::default()
            .stops(vec![
                StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0, 0).load(vec![1]).build_departure(),
                StopBuilder::default()
                    .coordinate((1., 0.))
                    .schedule_stamp(1, 2)
                    .load(vec![0])
                    .distance(1)
                    .build_single("d1", "delivery"),
                StopBuilder::default()
                    .coordinate((2., 0.))
                    .schedule_stamp(3, 4)
                    .load(vec![1])
                    .distance(2)
                    .build_single("p1", "pickup"),
                StopBuilder::default()
                    .coordinate((3., 0.))
                    .schedule_stamp(5, 7)
                    .load(vec![1])
                    .distance(3)
                    .build_single("reload", "reload"),
                StopBuilder::default()
                    .coordinate((4., 0.))
                    .schedule_stamp(8, 9)
                    .load(vec![0])
                    .distance(4)
                    .build_single("d2", "delivery"),
                StopBuilder::default()
                    .coordinate((5., 0.))
                    .schedule_stamp(10, 11)
                    .load(vec![1])
                    .distance(5)
                    .build_single("p2", "pickup"),
                StopBuilder::default()
                    .coordinate((6., 0.))
                    .schedule_stamp(12, 12)
                    .load(vec![0])
                    .distance(6)
                    .build_arrival(),
            ])
            .statistic(StatisticBuilder::default().driving(6).serving(6).build())
            .build()]
    );
    assert!(solution.violations.is_none());

    // NOTE reason can be sometimes NO_REASON_FOUND or CAPACITY_CONSTRAINT
    let unassigned = solution.unassigned.expect("no unassigned");
    assert_eq!(unassigned.len(), 1);
    assert_eq!(unassigned.first().unwrap().job_id, "d3");
}
