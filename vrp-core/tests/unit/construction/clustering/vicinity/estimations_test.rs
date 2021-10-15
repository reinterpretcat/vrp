use super::*;
use crate::construction::constraints::{ConstraintModule, ConstraintVariant};
use crate::helpers::models::problem::{SingleBuilder, TestTransportCost};
use std::slice::Iter;

struct VicinityTestModule {}

impl ConstraintModule for VicinityTestModule {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        unimplemented!()
    }

    fn accept_route_state(&self, ctx: &mut RouteContext) {
        unimplemented!()
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        unimplemented!()
    }

    fn merge_constrained(&self, _source: Job, _candidate: Job) -> Result<Job, i32> {
        todo!()
    }

    fn state_keys(&self) -> Iter<i32> {
        unimplemented!()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        unimplemented!()
    }
}

fn create_visit_info(
    service_time: Duration,
    forward: (Distance, Duration),
    backward: (Distance, Duration),
) -> VisitInfo {
    VisitInfo { service_time, forward, backward }
}

fn create_single_job(job_id: &str, places: Vec<(Option<Location>, Duration, Vec<(f64, f64)>)>) -> Job {
    SingleBuilder::default().id(job_id).places(places).build_as_job_ref()
}

fn create_default_config() -> ClusterConfig {
    ClusterConfig {
        threshold: ThresholdPolicy { moving_duration: 0.0, moving_distance: 0.0, min_shared_time: None },
        visiting: VisitPolicy::Repetition,
        service_time: ServiceTimePolicy::Original,
        filtering: FilterPolicy { job_filter: Arc::new(|_| true), actor_filter: Arc::new(|_| true) },
        building: BuilderPolicy {
            smallest_time_window: None,
            threshold: Arc::new(|_| true),
            ordering: Arc::new(|left, right| compare_floats(left.forward.1, right.forward.1)),
        },
    }
}

parameterized_test! {can_get_dissimilarities, (places_outer, places_inner, threshold, service_time, expected), {
    let threshold = ThresholdPolicy { moving_duration: threshold.0, moving_distance: threshold.1, min_shared_time: threshold.2 };
    let expected = expected.into_iter()
      .map(|e: (usize, usize, Duration, (Duration, Distance), (Duration, Distance))| (e.0, e.1, create_visit_info(e.2, e.3, e.4)))
      .collect();

    can_get_dissimilarities_impl(places_outer, places_inner, threshold, service_time, expected);
}}

can_get_dissimilarities! {
    case_01_one_place: (
        vec![(Some(1), 2., vec![(0., 10.)])],
        vec![(Some(2), 3., vec![(5., 15.)])],
        (5., 5., None), ServiceTimePolicy::Original,
        vec![(0, 0, 3., (1., 1.), (1., 1.))]
    ),
    case_02_two_places: (
        vec![(Some(1), 2., vec![(0., 10.)]), (Some(1), 3., vec![(20., 30.)])],
        vec![(Some(2), 3., vec![(5., 15.)]), (Some(2), 2., vec![(20., 40.)])],
        (5., 5., None), ServiceTimePolicy::Original,
        vec![(0, 0, 3., (1., 1.), (1., 1.)), (1, 1, 2., (1., 1.), (1., 1.))]
    ),
    case_03_two_places: (
        vec![(Some(1), 2., vec![(0., 10.)]), (Some(1), 3., vec![(20., 30.)])],
        vec![(Some(2), 3., vec![(5., 15.)]), (Some(2), 2., vec![(50., 60.)])],
        (5., 5., None), ServiceTimePolicy::Original,
        vec![(0, 0, 3., (1., 1.), (1., 1.))]
    ),

    case_04_service_time_policy: (
        vec![(Some(1), 2., vec![(0., 10.)])],
        vec![(Some(2), 3., vec![(5., 15.)])],
        (5., 5., None), ServiceTimePolicy::Multiplier(0.5),
        vec![(0, 0, 1.5, (1., 1.), (1., 1.))]
    ),
    case_05_service_time_policy: (
        vec![(Some(1), 2., vec![(0., 10.)])],
        vec![(Some(2), 3., vec![(5., 15.)])],
        (5., 5., None), ServiceTimePolicy::Fixed(20.),
        vec![(0, 0, 20., (1., 1.), (1., 1.))]
    ),

    case_06_threshold: (
        vec![(Some(1), 2., vec![(0., 10.)])],
        vec![(Some(5), 3., vec![(5., 15.)])],
        (2., 5., None), ServiceTimePolicy::Original,
        Vec::default(),
    ),
    case_07_threshold: (
        vec![(Some(1), 2., vec![(0., 10.)])],
        vec![(Some(5), 3., vec![(5., 15.)])],
        (5., 2., None), ServiceTimePolicy::Original,
        Vec::default(),
    ),

    case_08_shared_time: (
        vec![(Some(1), 2., vec![(0., 10.)])],
        vec![(Some(2), 3., vec![(5., 15.)])],
        (5., 5., Some(4.9)), ServiceTimePolicy::Original,
        vec![(0, 0, 3., (1., 1.), (1., 1.))]
    ),
    case_09_shared_time: (
        vec![(Some(1), 2., vec![(0., 10.)])],
        vec![(Some(2), 3., vec![(5., 15.)])],
        (5., 5., Some(5.)), ServiceTimePolicy::Original,
        Vec::default(),
    ),
    case_10_shared_time: (
        vec![(Some(1), 2., vec![(0., 10.)])],
        vec![(Some(2), 3., vec![(5., 15.)])],
        (5., 5., Some(6.)), ServiceTimePolicy::Original,
        Vec::default(),
    ),

    case_11_wide_time_windows: (
        vec![(Some(1), 2., vec![(0., 100.)])],
        vec![(Some(2), 3., vec![(5., 15.)]), (Some(5), 3., vec![(20., 40.)])],
        (5., 5., None), ServiceTimePolicy::Original,
        vec![(0, 0, 3., (1., 1.), (1., 1.)), (0, 1, 3., (4., 4.), (4., 4.))]
    ),
    case_12_wide_time_windows: (
        vec![(Some(1), 2., vec![(0., 10.)]), (Some(4), 2., vec![(20., 30.)])],
        vec![(Some(2), 3., vec![(0., 100.)])],
        (5., 5., None), ServiceTimePolicy::Original,
        vec![(0, 0, 3., (1., 1.), (1., 1.)), (1, 0, 3., (2., 2.), (2., 2.))]
    ),

    case_13_sorting_shared_time: (
        vec![(Some(1), 2., vec![(0., 100.)])],
        vec![(Some(2), 3., vec![(5., 15.), (20., 40.)])],
        (5., 5., Some(10.)), ServiceTimePolicy::Original,
        vec![(0, 0, 3., (1., 1.), (1., 1.))]
    ),
    case_14_sorting_shared_time: (
        vec![(Some(1), 2., vec![(0., 30.)])],
        vec![(Some(2), 3., vec![(5., 15.), (20., 40.)])],
        (5., 5., Some(10.)), ServiceTimePolicy::Original,
        Vec::default(),
    ),
}

fn can_get_dissimilarities_impl(
    places_outer: Vec<(Option<Location>, Duration, Vec<(f64, f64)>)>,
    places_inner: Vec<(Option<Location>, Duration, Vec<(f64, f64)>)>,
    threshold: ThresholdPolicy,
    service_time: ServiceTimePolicy,
    expected: Vec<(usize, usize, VisitInfo)>,
) {
    let outer = create_single_job("job1", places_outer);
    let inner = create_single_job("job2", places_inner);
    let transport = TestTransportCost::default();
    let profile = Profile::new(0, None);
    let config = ClusterConfig { threshold, service_time, ..create_default_config() };

    let dissimilarities = get_dissimilarities(&outer, &inner, &profile, &config, &transport);

    assert_eq!(dissimilarities.len(), expected.len());
    dissimilarities.into_iter().zip(expected.into_iter()).for_each(|(result, expected)| {
        assert_eq!(result.0, expected.0);
        assert_eq!(result.1, expected.1);
        assert_eq!(result.2.forward.0, expected.2.forward.0);
        assert_eq!(result.2.forward.1, expected.2.forward.1);
        assert_eq!(result.2.backward.0, expected.2.backward.0);
        assert_eq!(result.2.backward.1, expected.2.backward.1);
    });
}
