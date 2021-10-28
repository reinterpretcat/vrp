use super::*;
use crate::helpers::construction::clustering::vicinity::*;
use crate::helpers::models::problem::{get_job_id, SingleBuilder, TestTransportCost};

fn get_check_insertion_fn(disallow_insertion_list: Vec<&str>) -> Arc<CheckInsertionFn> {
    let disallow_insertion_list = disallow_insertion_list.into_iter().map(|id| id.to_string()).collect::<HashSet<_>>();

    Arc::new(move |job| {
        let job_to_check = job
            .dimens()
            .get_value::<Vec<Job>>(MERGED_KEY)
            .and_then(|merged| merged.last())
            .map(|last| last)
            .unwrap_or(job);
        let id = job_to_check.dimens().get_id();

        if id.map_or(false, |id| disallow_insertion_list.contains(id)) {
            Err(-1)
        } else {
            Ok(())
        }
    })
}

fn create_cluster_info(
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

fn compare_visit_info(result: &ClusterInfo, expected: &ClusterInfo) {
    assert_eq!(result.place_idx, expected.place_idx);
    assert_eq!(result.forward.0, expected.forward.0);
    assert_eq!(result.forward.1, expected.forward.1);
    assert_eq!(result.backward.0, expected.backward.0);
    assert_eq!(result.backward.1, expected.backward.1);
}

fn create_jobs(jobs_places: Vec<JobPlaces>) -> Vec<Job> {
    jobs_places
        .into_iter()
        .enumerate()
        .map(|(idx, places)| create_single_job(format!("job{}", idx + 1).as_str(), places))
        .collect()
}

fn get_location(job: &Job) -> Location {
    job.to_single().places.first().unwrap().location.unwrap()
}

parameterized_test! {can_get_dissimilarities, (places_outer, places_inner, threshold, service_time, expected), {
    let threshold = ThresholdPolicy { moving_duration: threshold.0, moving_distance: threshold.1, min_shared_time: threshold.2 };
    let expected = expected.into_iter()
      .map(|e: (usize, usize, Duration, (Duration, Distance), (Duration, Distance))| {
        let dummy_job = SingleBuilder::default().build_as_job_ref();
        (e.0, create_cluster_info(dummy_job, e.2, e.1, e.3, e.4))
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
    let config = ClusterConfig { threshold, service_time, ..create_cluster_config() };

    let dissimilarities = get_dissimilarities(&outer, &inner, &transport, &config)
        .into_iter()
        .filter(|(reachable, ..)| *reachable)
        .collect::<Vec<_>>();

    assert_eq!(dissimilarities.len(), expected.len());
    dissimilarities.into_iter().zip(expected.into_iter()).for_each(|(result, expected)| {
        assert_eq!(result.1, expected.0);
        compare_visit_info(&result.2, &expected.1);
    });
}

parameterized_test! {can_add_job, (center_places, candidate_places, is_disallowed_to_merge, is_disallowed_to_insert, visiting, smallest_time_window, expected), {
    let expected = expected.map(|e: (usize, Duration, (Duration, Distance), (Duration, Distance))| {
        let dummy_job = SingleBuilder::default().build_as_job_ref();
        create_cluster_info(dummy_job, e.1, e.0, e.2, e.3)
    });
    let building = create_cluster_config().building;
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
    center_places: JobPlaces,
    candidate_places: JobPlaces,
    is_disallowed_to_merge: bool,
    is_disallowed_to_insert: bool,
    visiting: VisitPolicy,
    building: BuilderPolicy,
    expected: Option<ClusterInfo>,
) {
    let config = ClusterConfig { visiting, building, ..create_cluster_config() };
    let cluster = create_single_job("cluster", center_places);
    let candidate = create_single_job("job1", candidate_places);
    let disallowed_merge = vec!["job1"];
    let disallowed_insert = vec!["job1"];
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
    let dissimilarity_info = get_dissimilarities(&cluster, &candidate, &transport, &config);
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
    can_build_job_cluster_impl(visiting, vec![], vec![], vec![], job_places, expected);
}}

can_build_job_cluster_with_policy! {
    case_01_closed: (VisitPolicy::ClosedContinuation, Some((vec![0, 1, 2, 3], 14., (0., 91.)))),
    case_02_open: (VisitPolicy::OpenContinuation, Some((vec![0, 1, 2, 3], 11., (0., 91.)))),
    // 100 -2s -1f 97 -2s -1b -2f 92 -2s -2b -3f 85 -2s -3b
    case_03_return: (VisitPolicy::Return, Some((vec![0, 1, 2, 3], 20., (0., 85.)))),
}

parameterized_test! {can_build_job_cluster_with_time_windows, (times, expected), {
    let job_places = vec![
        vec![(Some(1), 2., times.get(0).unwrap().clone())],
        vec![(Some(2), 2., times.get(1).unwrap().clone())],
        vec![(Some(3), 2., times.get(2).unwrap().clone())],
        vec![(Some(4), 2., times.get(3).unwrap().clone())],
    ];
    can_build_job_cluster_impl(VisitPolicy::ClosedContinuation, vec![], vec![], vec![], job_places, expected);
}}

can_build_job_cluster_with_time_windows! {
    case_01_same:   (vec![vec![(0., 100.)], vec![(0., 100.)], vec![(0., 100.)], vec![(0., 100.)]],
                     Some((vec![0, 1, 2, 3], 14., (0., 91.)))),
    case_02_diff:   (vec![vec![(0., 100.)], vec![(0., 50.)], vec![(0., 120.)], vec![(0., 100.)]],
                     Some((vec![0, 1, 2, 3], 14., (0., 41.)))),
    case_03_diff:   (vec![vec![(50., 100.)], vec![(20., 80.)], vec![(30., 60.)], vec![(0., 100.)]],
                     // NOTE ideally it can be: 54 +2s +1f 57 +2s +1f 60 ..
                     Some((vec![0, 1, 2, 3], 14., (50., 51.)))),

    case_04_skip:   (vec![vec![(20., 50.)], vec![(80., 100.)], vec![(0., 40.)], vec![(10., 30.)]],
                     // 23 +2s +2f 27 +1f +2s 30 ..
                     Some((vec![0, 2, 3], 12., (20., 23.)))),
    case_05_skip:   (vec![vec![(20., 50.)], vec![(80., 100.)], vec![(10., 30.)], vec![(0., 40.)]],
                     Some((vec![0, 2, 3], 12., (20., 23.)))),
    case_06_skip:   (vec![vec![(10., 30.)], vec![(80., 100.)], vec![(20., 50.)], vec![(0., 40.)]],
                     Some((vec![0, 2, 3], 12., (18., 23.)))),
    case_07_skip:   (vec![vec![(55., 100.)], vec![(20., 80.)], vec![(30., 60.)], vec![(0., 100.)]],
                     Some((vec![0, 1, 3], 12., (55., 73.)))),

    case_08_multi:  (vec![vec![(0., 40.)], vec![(100., 200.), (10., 30.)], vec![(20., 50.), (60., 80.)], vec![(0., 100.)]],
                     Some((vec![0, 1, 2, 3], 14., (19., 21.)))),

    case_09_shrink: (vec![vec![(0., 100.)], vec![(10., 90.)], vec![(20., 80.)], vec![(30., 70.)]],
                     Some((vec![0, 1, 2, 3], 14., (29., 61.)))),
    case_10_shrink: (vec![vec![(10., 90.)], vec![(20., 80.)], vec![(0., 100.)], vec![(30., 70.)]],
                     Some((vec![0, 1, 2, 3], 14., (29., 61.)))),
}

parameterized_test! {can_build_job_cluster_skipping_jobs, (merge, insertion, used_jobs, expected), {
    let job_places = vec![
        vec![(Some(1), 2., vec![(0., 100.)])],
        vec![(Some(2), 2., vec![(0., 100.)])],
        vec![(Some(3), 2., vec![(0., 100.)])],
        vec![(Some(4), 2., vec![(0., 100.)])],
    ];
    can_build_job_cluster_impl(VisitPolicy::ClosedContinuation, merge, insertion, used_jobs, job_places, expected);
}}

can_build_job_cluster_skipping_jobs! {
    case_01_empty:     (vec![], vec![], vec![], Some((vec![0, 1, 2, 3], 14., (0., 91.)))),
    case_02_merge:     (vec!["job2", "job4"], vec![], vec![], Some((vec![0, 2], 8., (0., 96.)))),
    case_03_insertion: (vec![], vec!["job2", "job4"], vec![], Some((vec![0, 2], 8., (0., 96.)))),
    case_04_all:       (vec!["job2", "job3", "job4"], vec![], vec![], None),
    case_05_used:      (vec![], vec![], vec![1, 3], Some((vec![0, 2], 8., (0., 96.)))),
}

fn can_build_job_cluster_impl(
    visiting: VisitPolicy,
    disallow_merge_list: Vec<&str>,
    disallow_insertion_list: Vec<&str>,
    used_jobs: Vec<usize>,
    jobs_places: Vec<JobPlaces>,
    expected: Option<(Vec<usize>, f64, (f64, f64))>,
) {
    let transport = TestTransportCost::default();
    let config = ClusterConfig { visiting, ..create_cluster_config() };
    let constraint = create_constraint_pipeline(disallow_merge_list);
    let check_insertion = get_check_insertion_fn(disallow_insertion_list);
    let jobs = create_jobs(jobs_places);
    let estimates = get_jobs_dissimilarities(jobs.as_slice(), &transport, &config);
    let used_jobs = used_jobs.iter().map(|idx| jobs.get(*idx).unwrap().clone()).collect();

    let result = build_job_cluster(
        &constraint,
        jobs.first().unwrap(),
        &estimates,
        &used_jobs,
        &config,
        check_insertion.as_ref(),
    );

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

parameterized_test! {can_get_clusters, (jobs_amount, moving_duration, expected), {
    can_get_clusters_impl(jobs_amount, moving_duration, expected);
}}

can_get_clusters! {
    case_01: (13, 2.5, vec![(8, vec![8, 9, 10, 7, 6]), (3, vec![3, 2, 1, 4, 5]), (12, vec![12, 11])]),
    case_02: (8, 2.5, vec![(5, vec![5, 4, 3, 6, 7]), (2, vec![2, 1, 0])]),
    case_03: (7, 2.5, vec![(4, vec![4, 3, 2, 5, 6]), (1, vec![1, 0])]),
    case_04: (6, 2.5, vec![(3, vec![3, 2, 1, 4, 5])]),
    case_05: (6, 3.5, vec![(3, vec![3, 2, 1, 0, 4, 5])]),
    case_06: (6, 0.5, vec![]),
}

pub fn can_get_clusters_impl(jobs_amount: usize, moving_duration: f64, expected: Vec<(usize, Vec<usize>)>) {
    let threshold = ThresholdPolicy { moving_duration, moving_distance: 10.0, min_shared_time: None };
    let disallow_merge_list = vec![];
    let disallow_insertion_list = vec![];
    let jobs_places = (0..jobs_amount).map(|idx| vec![(Some(idx), 2., vec![(0., 100.)])]).collect();
    let transport = TestTransportCost::default();
    let config = ClusterConfig { threshold, ..create_cluster_config() };
    let constraint = create_constraint_pipeline(disallow_merge_list);
    let check_insertion = get_check_insertion_fn(disallow_insertion_list);
    let jobs = create_jobs(jobs_places);
    let estimates = get_jobs_dissimilarities(jobs.as_slice(), &transport, &config);

    let result = get_clusters(&constraint, estimates, &config, check_insertion.as_ref());

    assert_eq!(result.len(), expected.len());
    let expected = expected
        .into_iter()
        .map(|(center, clustered)| {
            (jobs.get(center).unwrap(), clustered.into_iter().map(|idx| jobs.get(idx).unwrap()).collect::<Vec<_>>())
        })
        .collect::<Vec<_>>();
    result.into_iter().zip(expected).for_each(
        |((result_center, mut result_clustered), (expected_center, mut expected_clustered))| {
            assert_eq!(get_location(&result_center), get_location(expected_center));
            assert_eq!(result_clustered.len(), expected_clustered.len());

            assert_eq!(get_location(&result_center), get_location(expected_clustered.first().unwrap()));
            result_clustered.sort_by(|a, b| get_job_id(a).cmp(get_job_id(b)));
            expected_clustered.sort_by(|a, b| get_job_id(a).cmp(get_job_id(b)));
            result_clustered.iter().zip(expected_clustered.iter()).for_each(|(a, &b)| {
                assert!(a == b);
            });
        },
    );
}
