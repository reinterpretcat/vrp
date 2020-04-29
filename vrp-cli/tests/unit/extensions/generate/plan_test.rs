use super::*;

fn create_empty_job() -> Job {
    Job {
        id: "".to_string(),
        pickups: None,
        deliveries: None,
        replacements: None,
        services: None,
        priority: None,
        skills: None,
    }
}

fn create_empty_job_task() -> JobTask {
    JobTask { places: vec![], demand: None, tag: None }
}

fn create_empty_job_place() -> JobPlace {
    JobPlace { location: Location { lat: 0.0, lng: 0.0 }, duration: 0.0, times: None }
}

#[test]
fn can_generate_bounding_box() {
    let create_job_with_location = |lat: f64, lng: f64| Job {
        pickups: Some(vec![JobTask {
            places: vec![JobPlace { location: Location { lat, lng }, ..create_empty_job_place() }],
            ..create_empty_job_task()
        }]),
        ..create_empty_job()
    };
    let plan = Plan {
        jobs: vec![
            create_job_with_location(-1., 1.),
            create_job_with_location(1., 0.),
            create_job_with_location(3., 1.),
            create_job_with_location(1., 2.),
        ],
        relations: None,
    };

    let (Location { lat: min_lat, lng: min_lng }, Location { lat: max_lat, lng: max_lng }) =
        get_plan_bounding_box(&plan);
    assert_eq!(min_lat, -1.);
    assert_eq!(min_lng, 0.);
    assert_eq!(max_lat, 3.);
    assert_eq!(max_lng, 2.);
}
