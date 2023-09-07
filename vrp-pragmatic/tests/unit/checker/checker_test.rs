use super::*;
use crate::helpers::*;

#[test]
fn can_remove_duplicates_in_error_list() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", (2., 0.))], ..create_empty_plan() },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let solution = SolutionBuilder::default()
        .tour(
            TourBuilder::default()
                .vehicle_id("my_vehicle_11")
                .stops(vec![
                    StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0., 0.).load(vec![1]).build_departure(),
                    StopBuilder::default()
                        .coordinate((2., 0.))
                        .schedule_stamp(2., 3.)
                        .load(vec![0])
                        .distance(2)
                        .build_single("job1", "delivery"),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(5., 5.)
                        .load(vec![0])
                        .distance(4)
                        .build_arrival(),
                ])
                .statistic(StatisticBuilder::default().driving(18).serving(1).build())
                .build(),
        )
        .build();
    let core_problem = Arc::new(problem.clone().read_pragmatic().unwrap());

    let result = CheckerContext::new(core_problem, problem, None, solution).and_then(|ctx| ctx.check());

    assert_eq!(
        result,
        Err(vec![
            "cannot find vehicle with id 'my_vehicle_11'".into(),
            "used vehicle with unknown id: 'my_vehicle_11'".into()
        ])
    );
}
