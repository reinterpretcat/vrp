use super::*;
use crate::helpers::models::problem::test_single;
use crate::helpers::models::solution::*;
use crate::models::problem::Job;
use std::sync::Arc;

type RouteLeg = (Vec<usize>, usize);

fn get_memory_address(activity: &Activity) -> usize {
    (activity as *const Activity) as usize
}

fn get_test_tour() -> Tour {
    let mut tour = Tour::default();
    tour.set_start(test_activity_without_job());
    tour.set_end(test_activity_without_job());
    tour.insert_last(test_activity());
    tour.insert_last(test_activity());

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
    let activity = test_activity_without_job();
    let mut tour = Tour::default();

    tour.set_start(activity);
    let pointer = get_memory_address(&tour.activities[0]);

    assert_eq!(pointer, get_memory_address(tour.start().unwrap()));
    assert_eq!(tour.activity_count(), 0);
    assert_eq!(tour.job_count(), 0);
}

#[test]
fn can_set_and_get_end() {
    let activity = test_activity_without_job();
    let mut tour = Tour::default();
    tour.set_start(test_activity_without_job());

    tour.set_end(activity);
    let pointer = get_memory_address(&tour.activities[1]);

    assert_eq!(pointer, get_memory_address(tour.end().unwrap()));
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
    let activity = test_activity_with_location(42);
    let mut tour = get_test_tour();

    tour.insert_at(activity, position);

    assert_eq!(tour.get(position).unwrap().place.location, 42);
}

#[test]
fn can_insert_at_last_position() {
    let activity = test_activity_with_location(42);
    let mut tour = get_test_tour();

    tour.insert_last(activity);

    assert_eq!(tour.get(3).unwrap().place.location, 42);
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
    let activity = test_activity_with_job(job.clone());

    tour.insert_at(activity, 2);
    let pointer = get_memory_address(&tour.activities[2]);

    let job = Job::Single(job);

    let result: Vec<&Activity> = tour.job_activities(&job).collect();

    assert_eq!(result.len(), 1);
    assert_eq!(pointer, get_memory_address(result.first().unwrap()))
}

#[test]
fn can_get_legs() {
    let start = test_activity_without_job();
    let end = test_activity_without_job();
    let a1 = test_activity();
    let a2 = test_activity();

    // s a1 a2 e
    let mut tour = Tour::default();
    tour.set_start(start);
    tour.set_end(end);
    tour.insert_last(a1);
    tour.insert_last(a2);

    let legs: Vec<(Vec<usize>, usize)> =
        tour.legs().map(|(leg, index)| (leg.iter().map(get_memory_address).collect(), index)).collect();

    let start_ptr = get_memory_address(&tour.activities[0]);
    let end_ptr = get_memory_address(&tour.activities[3]);
    let a1_ptr = get_memory_address(&tour.activities[1]);
    let a2_ptr = get_memory_address(&tour.activities[2]);

    // (s,a1) (a1,a2) (a2,e)
    assert_eq!(legs.len(), 3);
    compare_legs(legs.get(0).unwrap(), &(vec![start_ptr, a1_ptr], 0));
    compare_legs(legs.get(1).unwrap(), &(vec![a1_ptr, a2_ptr], 1));
    compare_legs(legs.get(2).unwrap(), &(vec![a2_ptr, end_ptr], 2));
}

#[test]
fn can_get_job_index() {
    let mut tour = Tour::default();
    tour.set_start(test_activity_without_job());
    tour.set_end(test_activity_without_job());
    let job = Arc::new(test_single());
    tour.insert_last(test_activity());
    tour.insert_last(test_activity_with_job(job.clone()));
    tour.insert_last(test_activity());
    tour.insert_last(test_activity_with_job(job.clone()));
    tour.insert_last(test_activity());

    let index = tour.index(&Job::Single(job));

    assert_eq!(index.unwrap(), 2);
    assert_eq!(tour.job_count(), 4);
}

#[test]
fn can_get_activity_and_job_count() {
    let mut tour = Tour::default();

    tour.set_start(test_activity_without_job());
    assert_eq!(tour.activity_count(), 0);
    assert_eq!(tour.job_count(), 0);

    tour.set_end(test_activity_without_job());
    assert_eq!(tour.activity_count(), 0);
    assert_eq!(tour.job_count(), 0);

    tour.insert_last(test_activity());
    assert_eq!(tour.activity_count(), 1);
    assert_eq!(tour.job_count(), 1);
}

#[test]
fn can_get_start_and_end() {
    let mut tour = Tour::default();

    tour.set_start(test_activity_without_job());
    tour.set_end(test_activity_without_job());
    tour.insert_last(test_activity());
    tour.insert_last(test_activity());

    assert_eq!(get_memory_address(tour.start().unwrap()), get_memory_address(&tour.activities[0]));
    assert_eq!(get_memory_address(tour.end().unwrap()), get_memory_address(&tour.activities[3]));
}
