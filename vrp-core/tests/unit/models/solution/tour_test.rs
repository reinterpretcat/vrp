use crate::helpers::models::problem::test_single;
use crate::helpers::models::solution::*;
use crate::models::problem::Job;
use crate::models::solution::{Activity, Tour, TourActivity};
use std::ops::Deref;
use std::sync::Arc;

type RouteLeg = (Vec<*const Activity>, usize);

fn get_pointer(activity: &TourActivity) -> *const Activity {
    activity.deref() as *const Activity
}

fn get_test_tour() -> Tour {
    let mut tour = Tour::default();
    tour.set_start(test_tour_activity_without_job());
    tour.set_end(test_tour_activity_without_job());
    tour.insert_last(test_tour_activity_with_default_job());
    tour.insert_last(test_tour_activity_with_default_job());

    tour
}

fn compare_legs(left: &RouteLeg, right: &RouteLeg) {
    for i in 0..2 {
        assert_eq!(left.0.get(i).unwrap(), right.0.get(i).unwrap());
    }
    assert_eq!(left.1, right.1);
}

#[test]
fn can_set_and_get_start() {
    let activity = test_tour_activity_without_job();
    let mut tour = Tour::default();
    let pointer = get_pointer(&activity);

    tour.set_start(activity);

    assert_eq!(pointer, get_pointer(tour.start().unwrap()));
    assert_ne!(get_pointer(&test_tour_activity_without_job()), get_pointer(tour.start().unwrap()));
    assert_eq!(tour.activity_count(), 0);
    assert_eq!(tour.job_count(), 0);
}

#[test]
fn can_set_and_get_end() {
    let activity = test_tour_activity_without_job();
    let mut tour = Tour::default();
    tour.set_start(test_tour_activity_without_job());
    let pointer = get_pointer(&activity);

    tour.set_end(activity);

    assert_eq!(pointer, get_pointer(tour.end().unwrap()));
    assert_ne!(get_pointer(&test_tour_activity_without_job()), get_pointer(tour.end().unwrap()));
    assert_eq!(tour.activity_count(), 0);
    assert_eq!(tour.job_count(), 0);
}

parameterized_test! {can_insert_at_specific_position, position, {
    can_insert_at_specific_position_impl(position);
}}

can_insert_at_specific_position! {
    case1: 0,
    case2: 1,
    case3: 2,
    case4: 3,
    case5: 4,
}

fn can_insert_at_specific_position_impl(position: usize) {
    let activity = test_tour_activity_with_default_job();
    let mut tour = get_test_tour();
    let pointer = get_pointer(&activity);

    tour.insert_at(activity, position);

    assert_eq!(pointer, get_pointer(tour.get(position).unwrap()));
}

#[test]
fn can_insert_at_last_position() {
    let activity = test_tour_activity_with_default_job();
    let mut tour = get_test_tour();
    let pointer = get_pointer(&activity);

    tour.insert_last(activity);

    assert_eq!(pointer, get_pointer(tour.get(3).unwrap()));
}

#[test]
fn can_remove_job() {
    let mut tour = get_test_tour();
    let job = tour.jobs().last().unwrap();
    assert_eq!(tour.job_count(), 2);

    let removed = tour.remove(&job);

    assert!(removed);
    assert_eq!(tour.job_count(), 1);
}

#[test]
fn can_get_activities_for_job() {
    let mut tour = get_test_tour();
    let job = Arc::new(test_single());
    let activity = test_tour_activity_with_job(job.clone());
    let pointer = get_pointer(&activity);
    tour.insert_at(activity, 2);
    let job = Job::Single(job);

    let result: Vec<&TourActivity> = tour.job_activities(&job).collect();

    assert_eq!(result.len(), 1);
    assert_eq!(pointer, get_pointer(result.first().unwrap()))
}

#[test]
fn can_get_legs() {
    let start = test_tour_activity_without_job();
    let end = test_tour_activity_without_job();
    let a1 = test_tour_activity_with_default_job();
    let a2 = test_tour_activity_with_default_job();

    let start_ptr = get_pointer(&start);
    let end_ptr = get_pointer(&end);
    let a1_ptr = get_pointer(&a1);
    let a2_ptr = get_pointer(&a2);
    // s a1 a2 e
    let mut tour = Tour::default();
    tour.set_start(start);
    tour.set_end(end);
    tour.insert_last(a1);
    tour.insert_last(a2);

    let legs: Vec<(Vec<*const Activity>, usize)> =
        tour.legs().map(|(leg, index)| (leg.iter().map(|a| a.deref() as *const Activity).collect(), index)).collect();

    // (s,a1) (a1,a2) (a2,e)
    assert_eq!(legs.len(), 3);
    compare_legs(legs.get(0).unwrap(), &(vec![start_ptr, a1_ptr], 0));
    compare_legs(legs.get(1).unwrap(), &(vec![a1_ptr, a2_ptr], 1));
    compare_legs(legs.get(2).unwrap(), &(vec![a2_ptr, end_ptr], 2));
}

#[test]
fn can_get_job_index() {
    let mut tour = Tour::default();
    tour.set_start(test_tour_activity_without_job());
    tour.set_end(test_tour_activity_without_job());
    let job = Arc::new(test_single());
    tour.insert_last(test_tour_activity_with_default_job());
    tour.insert_last(test_tour_activity_with_job(job.clone()));
    tour.insert_last(test_tour_activity_with_default_job());
    tour.insert_last(test_tour_activity_with_job(job.clone()));
    tour.insert_last(test_tour_activity_with_default_job());

    let index = tour.index(&Job::Single(job));

    assert_eq!(index.unwrap(), 2);
    assert_eq!(tour.job_count(), 4);
}

#[test]
fn can_get_activity_and_job_count() {
    let mut tour = Tour::default();

    tour.set_start(test_tour_activity_without_job());
    assert_eq!(tour.activity_count(), 0);
    assert_eq!(tour.job_count(), 0);

    tour.set_end(test_tour_activity_without_job());
    assert_eq!(tour.activity_count(), 0);
    assert_eq!(tour.job_count(), 0);

    tour.insert_last(test_tour_activity_with_default_job());
    assert_eq!(tour.activity_count(), 1);
    assert_eq!(tour.job_count(), 1);
}

#[test]
fn can_get_start_and_end() {
    let start = test_tour_activity_without_job();
    let end = test_tour_activity_without_job();
    let a1 = test_tour_activity_with_default_job();
    let a2 = test_tour_activity_with_default_job();
    let start_ptr = get_pointer(&start);
    let end_ptr = get_pointer(&end);
    let mut tour = Tour::default();

    tour.set_start(start);
    tour.set_end(end);
    tour.insert_last(a1);
    tour.insert_last(a2);

    assert_eq!(start_ptr, get_pointer(tour.start().unwrap()));
    assert_eq!(end_ptr, get_pointer(tour.end().unwrap()));
}
