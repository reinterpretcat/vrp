use super::*;
use crate::helpers::get_test_resource;
use std::io::BufReader;

fn assert_time_windows(actual: &Option<Vec<Vec<String>>>, expected: (&str, &str)) {
    let actual = actual.as_ref().unwrap();
    assert_eq!(actual.len(), 1);
    assert_eq!(actual.first().unwrap().len(), 2);
    assert_eq!(actual.first().unwrap().first().unwrap(), expected.0);
    assert_eq!(actual.first().unwrap().last().unwrap(), expected.1);
}

fn assert_location(actual: &Location, expected: (f64, f64)) {
    assert_eq!(actual.lat, expected.0);
    assert_eq!(actual.lng, expected.1);
}

fn assert_demand(actual: &Vec<i32>, expected: i32) {
    assert_eq!(actual.len(), 1);
    assert_eq!(*actual.first().unwrap(), expected);
}

#[test]
fn can_deserialize_problem() {
    let file = get_test_resource("../data/small/minimal.problem.json").unwrap();

    let problem = deserialize_problem(BufReader::new(file)).unwrap();

    assert_eq!(problem.plan.jobs.len(), 2);
    assert_eq!(problem.fleet.vehicles.len(), 1);
    assert!(problem.plan.relations.is_none());

    // validate jobs
    let job = problem.plan.jobs.first().unwrap();
    assert_eq!(job.id, "single_job");
    assert!(job.requirement.pickups.is_none());
    assert!(job.requirement.deliveries.is_some());
    assert!(job.skills.is_none());

    let deliveries = job.requirement.deliveries.as_ref().unwrap();
    assert_eq!(deliveries.len(), 1);
    let delivery = deliveries.first().unwrap();
    assert_demand(&delivery.demand, 1);
    assert!(delivery.tag.is_none());

    assert_eq!(delivery.places.len(), 1);
    let place = delivery.places.first().unwrap();
    assert_eq!(place.duration, 240.);
    assert_location(&place.location, (52.5622847f64, 13.4023099f64));
    assert_time_windows(&place.times, ("2019-07-04T10:00:00Z", "2019-07-04T16:00:00Z"));

    let job = problem.plan.jobs.last().unwrap();
    assert_eq!(job.id, "multi_job");
    assert!(job.skills.is_none());
    assert_eq!(job.requirement.pickups.as_ref().unwrap().len(), 2);
    assert_eq!(job.requirement.deliveries.as_ref().unwrap().len(), 1);
}

#[test]
fn can_deserialize_matrix() {
    let file = get_test_resource("../data/small/minimal.matrix.json").unwrap();

    let matrix = deserialize_matrix(BufReader::new(file)).unwrap();

    assert_eq!(matrix.num_origins, 4);
    assert_eq!(matrix.num_destinations, 4);
    assert_eq!(matrix.distances.len(), 16);
    assert_eq!(matrix.travel_times.len(), 16);
}
