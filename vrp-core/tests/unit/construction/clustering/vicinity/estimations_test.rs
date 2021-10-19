use super::*;
use crate::construction::constraints::{ConstraintModule, ConstraintVariant};
use crate::helpers::models::problem::{SingleBuilder, TestTransportCost};
use std::slice::Iter;

struct VicinityTestModule {
    disallow_merge_list: HashSet<String>,
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
}

impl VicinityTestModule {
    pub fn new(disallow_merge_list: HashSet<String>) -> Self {
        Self { disallow_merge_list, constraints: Vec::default(), keys: Vec::default() }
    }
}

impl ConstraintModule for VicinityTestModule {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {
        unimplemented!()
    }

    fn accept_route_state(&self, _: &mut RouteContext) {
        unimplemented!()
    }

    fn accept_solution_state(&self, _: &mut SolutionContext) {
        unimplemented!()
    }

    fn merge_constrained(&self, source: Job, candidate: Job) -> Result<Job, i32> {
        if self.disallow_merge_list.contains(candidate.dimens().get_id().unwrap()) {
            Err(1)
        } else {
            let source = source.to_single();
            assert_eq!(source.places.len(), 1);

            let place = source.places.first().unwrap();
            let place = (
                place.location,
                place.duration,
                place
                    .times
                    .iter()
                    .map(|t| {
                        let tw = t.as_time_window().unwrap();
                        (tw.start, tw.end)
                    })
                    .collect::<Vec<_>>(),
            );

            Ok(SingleBuilder::default().dimens(source.dimens.clone()).places(vec![place]).build_as_job_ref())
        }
    }

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

fn create_constraint_pipeline(disallow_merge_list: Vec<String>) -> ConstraintPipeline {
    let mut pipeline = ConstraintPipeline::default();

    let disallow_merge_list = disallow_merge_list.into_iter().collect();

    pipeline.add_module(Arc::new(VicinityTestModule::new(disallow_merge_list)));

    pipeline
}

fn get_check_insertion_fn(disallow_insertion_list: Vec<String>) -> Arc<dyn Fn(&Job) -> bool + Send + Sync> {
    let disallow_insertion_list = disallow_insertion_list.into_iter().collect::<HashSet<_>>();

    Arc::new(move |job| !disallow_insertion_list.contains(job.dimens().get_id().unwrap()))
}

fn create_visit_info(
    job: Job,
    service_time: Duration,
    place_idx: usize,
    forward: (Distance, Duration),
    backward: (Distance, Duration),
) -> ClusterInfo {
    ClusterInfo { job, service_time, place_idx, forward, backward }
}

fn create_single_job(job_id: &str, places: Vec<(Option<Location>, Duration, Vec<(f64, f64)>)>) -> Job {
    SingleBuilder::default().id(job_id).places(places).build_as_job_ref()
}

fn create_default_config() -> ClusterConfig {
    ClusterConfig {
        threshold: ThresholdPolicy { moving_duration: 10.0, moving_distance: 10.0, min_shared_time: None },
        visiting: VisitPolicy::Return,
        service_time: ServiceTimePolicy::Original,
        filtering: FilterPolicy { job_filter: Arc::new(|_| true), actor_filter: Arc::new(|_| true) },
        building: BuilderPolicy {
            smallest_time_window: None,
            threshold: Arc::new(|_| true),
            ordering: Arc::new(|left, right| compare_floats(left.forward.1, right.forward.1)),
        },
    }
}

fn compare_visit_info(result: &ClusterInfo, expected: &ClusterInfo) {
    assert_eq!(result.place_idx, expected.place_idx);
    assert_eq!(result.forward.0, expected.forward.0);
    assert_eq!(result.forward.1, expected.forward.1);
    assert_eq!(result.backward.0, expected.backward.0);
    assert_eq!(result.backward.1, expected.backward.1);
}

parameterized_test! {can_get_dissimilarities, (places_outer, places_inner, threshold, service_time, expected), {
    let threshold = ThresholdPolicy { moving_duration: threshold.0, moving_distance: threshold.1, min_shared_time: threshold.2 };
    let expected = expected.into_iter()
      .map(|e: (usize, usize, Duration, (Duration, Distance), (Duration, Distance))| {
        let dummy_job = SingleBuilder::default().build_as_job_ref();
        (e.0, create_visit_info(dummy_job, e.2, e.1, e.3, e.4))
      })
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
    expected: Vec<(usize, ClusterInfo)>,
) {
    let outer = create_single_job("job1", places_outer);
    let inner = create_single_job("job2", places_inner);
    let transport = TestTransportCost::default();
    let profile = Profile::new(0, None);
    let config = ClusterConfig { threshold, service_time, ..create_default_config() };

    let dissimilarities = get_dissimilarities(&outer, &inner, &profile, &transport, &config);

    assert_eq!(dissimilarities.len(), expected.len());
    dissimilarities.into_iter().zip(expected.into_iter()).for_each(|(result, expected)| {
        assert_eq!(result.0, expected.0);
        compare_visit_info(&result.1, &expected.1);
    });
}

parameterized_test! {can_add_job, (center_places, candidate_places, is_disallowed_to_merge, is_disallowed_to_insert, visiting, smallest_time_window, expected), {
    let expected = expected.map(|e: (usize, Duration, (Duration, Distance), (Duration, Distance))| {
        let dummy_job = SingleBuilder::default().build_as_job_ref();
        create_visit_info(dummy_job, e.1, e.0, e.2, e.3)
    });
    let building = create_default_config().building;
    let building = BuilderPolicy { smallest_time_window, ..building };

    can_add_job_impl(center_places, candidate_places, is_disallowed_to_merge, is_disallowed_to_insert, visiting, building, expected);
}}

can_add_job! {
    case_01_trivial: (
        vec![(Some(1), 2., vec![(0., 100.)])], vec![(Some(5), 2., vec![(0., 100.)])],
        false, false, VisitPolicy::ClosedContinuation, None, Some((0, 4., (4., 4.), (4., 4.))),
    ),
    case_02_two_places: (
        vec![(Some(1), 2., vec![(0., 100.)])], vec![(Some(5), 2., vec![(0., 100.)]), (Some(3), 3., vec![(0., 100.)])],
        false, false, VisitPolicy::ClosedContinuation, None,Some((1, 5., (2., 2.), (2., 2.))),
    ),

    case_03_disallowed_insertion: (
        vec![(Some(1), 2., vec![(0., 100.)])], vec![(Some(5), 2., vec![(0., 100.)])],
        false, true, VisitPolicy::ClosedContinuation, None, None,
    ),
    case_04_disallowed_merge: (
        vec![(Some(1), 2., vec![(0., 100.)])], vec![(Some(5), 2., vec![(0., 100.)])],
        true, false, VisitPolicy::ClosedContinuation, None, None,
    ),

    case_05_visit_repetition: (
        vec![(Some(1), 2., vec![(0., 100.)])], vec![(Some(5), 2., vec![(0., 100.)])],
        false, false, VisitPolicy::Return, None, Some((0, 8., (4., 4.), (4., 4.))),
    ),

    case_06_time_window_threshold_above: (
        vec![(Some(1), 2., vec![(0., 100.)])], vec![(Some(5), 2., vec![(0., 100.)])],
        false, false, VisitPolicy::ClosedContinuation, Some(101.), None,
    ),
    case_07_time_window_threshold_below: (
        vec![(Some(1), 2., vec![(0., 100.)])], vec![(Some(5), 2., vec![(0., 100.)])],
        false, false, VisitPolicy::ClosedContinuation, Some(94.), Some((0, 4., (4., 4.), (4., 4.))),
    ),
}

fn can_add_job_impl(
    center_places: Vec<(Option<Location>, Duration, Vec<(f64, f64)>)>,
    candidate_places: Vec<(Option<Location>, Duration, Vec<(f64, f64)>)>,
    is_disallowed_to_merge: bool,
    is_disallowed_to_insert: bool,
    visiting: VisitPolicy,
    building: BuilderPolicy,
    expected: Option<ClusterInfo>,
) {
    let config = ClusterConfig { visiting, building, ..create_default_config() };
    let cluster = create_single_job("cluster", center_places);
    let candidate = create_single_job("job1", candidate_places);
    let disallowed_merge = vec!["job1".to_string()];
    let disallowed_insert = vec!["cluster".to_string()];
    let (disallow_merge_list, disallow_insertion_list) = match (is_disallowed_to_merge, is_disallowed_to_insert) {
        (true, true) => (disallowed_merge.clone(), disallowed_insert),
        (true, false) => (disallowed_merge, Vec::default()),
        (false, true) => (Vec::default(), disallowed_insert),
        (false, false) => (Vec::default(), Vec::default()),
    };
    let constraint = create_constraint_pipeline(disallow_merge_list);
    let check_insertion = get_check_insertion_fn(disallow_insertion_list);
    let return_movement = |info: &ClusterInfo| (info.forward.clone(), info.backward.clone());
    let transport = TestTransportCost::default();
    let profile = Profile::new(0, None);
    let dissimilarity_info = get_dissimilarities(&cluster, &candidate, &profile, &transport, &config);
    let candidate = (&candidate, &dissimilarity_info);

    let result = try_add_job(&constraint, 0, &cluster, candidate, &config, &return_movement, check_insertion.as_ref());

    match (result, expected) {
        (Some((_, result_visit_info)), Some(expected_visit_info)) => {
            assert_eq!(result_visit_info.place_idx, expected_visit_info.place_idx);
            compare_visit_info(&result_visit_info, &expected_visit_info);
        }
        (Some(_), None) => unreachable!("unexpected some result"),
        (None, Some(_)) => unreachable!("unexpected none result"),
        (None, None) => {}
    }
}

parameterized_test! {can_build_job_cluster_with_policy, (visiting, expected), {
    let job_places = vec![
        vec![(Some(1), 2., vec![(0., 100.)])],
        vec![(Some(2), 2., vec![(0., 100.)])],
        vec![(Some(3), 2., vec![(0., 100.)])],
        vec![(Some(4), 2., vec![(0., 100.)])],
    ];
    can_build_job_cluster_impl(visiting, vec![], vec![], job_places, expected);
}}

can_build_job_cluster_with_policy! {
    case_01_closed: (VisitPolicy::ClosedContinuation, Some((vec![0, 1, 2, 3], 14., (0., 91.)))),
    case_02_open: (VisitPolicy::OpenContinuation, Some((vec![0, 1, 2, 3], 11., (0., 91.)))),
    // 100 -2s -1f 97 -2s -1b -2f 92 -2s -2b -3f 85 -2s -3b
    case_03_return: (VisitPolicy::Return, Some((vec![0, 1, 2, 3], 20., (0., 85.)))),
}

fn can_build_job_cluster_impl(
    visiting: VisitPolicy,
    disallow_merge_list: Vec<String>,
    disallow_insertion_list: Vec<String>,
    jobs_places: Vec<Vec<(Option<Location>, Duration, Vec<(f64, f64)>)>>,
    expected: Option<(Vec<usize>, f64, (f64, f64))>,
) {
    let transport = TestTransportCost::default();
    let profile = Profile::new(0, None);
    let config = ClusterConfig { visiting, ..create_default_config() };
    let constraint = create_constraint_pipeline(disallow_merge_list);
    let check_insertion = get_check_insertion_fn(disallow_insertion_list);
    let jobs = jobs_places
        .into_iter()
        .enumerate()
        .map(|(idx, places)| create_single_job(format!("job{}", idx).as_str(), places))
        .collect::<Vec<_>>();
    let estimates = get_jobs_dissimilarities(jobs.as_slice(), &profile, &transport, &config);

    let result = build_job_cluster(&constraint, jobs.first().unwrap(), &estimates, &config, check_insertion.as_ref());

    match (result, expected) {
        (Some(result), Some((expected_indices, expected_duration, expected_time))) => {
            let result_job = result.to_single();
            assert_eq!(result_job.places.len(), 1);
            let result_place = result_job.places.first().unwrap();

            assert_eq!(result_place.times.len(), 1);
            let times = filter_times(result_place.times.as_slice());
            assert_eq!(times.len(), 1);
            let time = times.first().unwrap();
            assert_eq!(time.start, expected_time.0);
            assert_eq!(time.end, expected_time.1);

            assert_eq!(result_place.duration, expected_duration);

            let result_clustered_jobs =
                result_job.dimens.get_cluster().unwrap().into_iter().map(|info| info.job.clone()).collect::<Vec<_>>();
            let expected_jobs = expected_indices.into_iter().map(|idx| jobs.get(idx).unwrap()).collect::<Vec<_>>();
            assert_eq!(result_clustered_jobs.len(), expected_jobs.len());
            result_clustered_jobs.iter().zip(expected_jobs.iter()).for_each(|(a, &b)| {
                assert!(a == b);
            });
        }
        (Some(_), None) => unreachable!("unexpected some result"),
        (None, Some(_)) => unreachable!("unexpected none result"),
        (None, None) => {}
    }
}
