use crate::helpers::models::problem::test_single_job;
use crate::helpers::models::solution::{
    test_tour_activity_with_default_job, test_tour_activity_with_job,
    test_tour_activity_without_job,
};
use crate::models::problem::Job;
use crate::models::solution::{Activity, Tour};
use std::borrow::Borrow;
use std::sync::Arc;

type RouteLeg<'a> = (&'a [Arc<Activity>], usize);

fn get_pointer(activity: &Arc<Activity>) -> *const Activity {
    &*activity.borrow() as *const Activity
}

fn get_test_tour() -> Tour {
    let mut tour = Tour::new();
    tour.set_start(test_tour_activity_without_job());
    tour.set_end(test_tour_activity_without_job());
    tour.insert_last(test_tour_activity_with_default_job());
    tour.insert_last(test_tour_activity_with_default_job());

    tour
}

fn compare_legs<'a>(left: &RouteLeg<'a>, right: &RouteLeg<'a>) {
    for i in 0..2 {
        assert_eq!(
            get_pointer(left.0.get(i).unwrap()),
            get_pointer(right.0.get(i).unwrap())
        );
    }
    assert_eq!(left.1, right.1);
}

#[test]
fn can_set_and_get_start() {
    let activity = test_tour_activity_without_job();
    let mut tour = Tour::new();

    tour.set_start(activity.clone());

    assert_eq!(get_pointer(&activity), get_pointer(tour.start().unwrap()));
    assert_ne!(
        get_pointer(&test_tour_activity_without_job()),
        get_pointer(tour.start().unwrap())
    );
    assert_eq!(tour.activity_count(), 0);
    assert_eq!(tour.job_count(), 0);
}

#[test]
fn can_set_and_get_end() {
    let activity = test_tour_activity_without_job();
    let mut tour = Tour::new();
    tour.set_start(test_tour_activity_without_job());

    tour.set_end(activity.clone());

    assert_eq!(get_pointer(&activity), get_pointer(tour.end().unwrap()));
    assert_ne!(
        get_pointer(&test_tour_activity_without_job()),
        get_pointer(tour.end().unwrap())
    );
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

    tour.insert_at(activity.clone(), position);

    assert_eq!(
        get_pointer(&activity),
        get_pointer(tour.get(position).unwrap())
    );
}

#[test]
fn can_insert_at_last_position() {
    let activity = test_tour_activity_with_default_job();
    let mut tour = get_test_tour();

    tour.insert_last(activity.clone());

    assert_eq!(get_pointer(&activity), get_pointer(tour.get(3).unwrap()));
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
    let job = Arc::new(test_single_job());
    let activity = test_tour_activity_with_job(job.clone());
    tour.insert_at(activity.clone(), 2);

    let result: Vec<Arc<Activity>> = tour.activities(&job).collect();

    assert_eq!(result.len(), 1);
    assert_eq!(get_pointer(&activity), get_pointer(result.first().unwrap()))
}

#[test]
fn can_get_legs() {
    let start = test_tour_activity_without_job();
    let end = test_tour_activity_without_job();
    let a1 = test_tour_activity_with_default_job();
    let a2 = test_tour_activity_with_default_job();
    // s a1 a2 e
    let mut tour = Tour::new();
    tour.set_start(start.clone());
    tour.set_end(end.clone());
    tour.insert_last(a1.clone());
    tour.insert_last(a2.clone());

    let legs = tour.legs().collect::<Vec<RouteLeg>>();

    // (s,a1) (a1,a2) (a2,e)
    assert_eq!(legs.len(), 3);
    compare_legs(legs.get(0).unwrap(), &(&[start.clone(), a1.clone()], 0));
    compare_legs(legs.get(1).unwrap(), &(&[a1.clone(), a2.clone()], 1));
    compare_legs(legs.get(2).unwrap(), &(&[a2.clone(), end.clone()], 2));
}

#[test]
fn can_get_job_index() {
    let mut tour = Tour::new();
    tour.set_start(test_tour_activity_without_job());
    tour.set_end(test_tour_activity_without_job());
    let job = Arc::new(test_single_job());
    tour.insert_last(test_tour_activity_with_default_job());
    tour.insert_last(test_tour_activity_with_job(job.clone()));
    tour.insert_last(test_tour_activity_with_default_job());
    tour.insert_last(test_tour_activity_with_job(job.clone()));
    tour.insert_last(test_tour_activity_with_default_job());

    let index = tour.index(&job);

    assert_eq!(index.unwrap(), 2);
    assert_eq!(tour.job_count(), 4);
}

#[test]
fn can_get_activity_and_job_count() {
    let mut tour = Tour::new();

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
    let mut tour = Tour::new();
    tour.set_start(start.clone());
    tour.set_end(end.clone());
    tour.insert_last(a1.clone());
    tour.insert_last(a2.clone());

    assert_eq!(get_pointer(&start), get_pointer(tour.start().unwrap()));
    assert_eq!(get_pointer(&end), get_pointer(tour.end().unwrap()));
}
