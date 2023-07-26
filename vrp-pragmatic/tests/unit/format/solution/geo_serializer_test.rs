use super::*;
use crate::format::problem::Problem as FormatProblem;
use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_create_geo_json_from_solution() {
    let problem = FormatProblem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job_with_demand("job2", (2., 0.), vec![10]),
                create_delivery_job("job3", (3., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);
    let core_problem = (problem.clone(), vec![matrix.clone()]).read_pragmatic().unwrap();
    let solution = solve_with_cheapest_insertion(problem, Some(vec![matrix]));
    let geo_json = create_feature_collection(&core_problem, &solution).unwrap();

    assert_eq!(geo_json.features.len(), 6);
}

#[test]
fn can_create_geo_json_from_named_locations() {
    let locations = vec![
        ("job1".to_string(), Location::Coordinate { lat: 1.0, lng: 0.0 }, 0),
        ("job2".to_string(), Location::Coordinate { lat: 2.0, lng: 0.0 }, 0),
        ("job3".to_string(), Location::Coordinate { lat: 3.0, lng: 0.0 }, 1),
    ];
    let create_feature = |name: &str, coordinates: (f64, f64), color: &str| Feature {
        properties: slice_to_map(&[
            ("marker-color", color),
            ("marker-size", "medium"),
            ("marker-symbol", "marker"),
            ("name", name),
        ]),
        geometry: Geometry::Point { coordinates },
    };

    let geo_json = create_geojson_named_locations(locations.as_slice()).unwrap();

    assert_eq!(
        geo_json,
        FeatureCollection {
            features: vec![
                create_feature("job1", (0., 1.), "#000000"),
                create_feature("job2", (0., 2.), "#000000"),
                create_feature("job3", (0., 3.), "#FFFF00"),
            ]
        }
    );
}

#[test]
fn can_create_geo_json_for_cluster_geometry() {
    let stop = PointStop {
        location: Location::Coordinate { lat: 1., lng: 0. },
        time: Schedule { arrival: format_time(0.), departure: format_time(10.) },
        distance: 0,
        load: vec![],
        parking: None,
        activities: vec![
            Activity {
                job_id: "job1".to_string(),
                activity_type: "delivery".to_string(),
                location: Some(Location::Coordinate { lat: 1., lng: 0.0 }),
                time: Some(Interval { start: format_time(0.), end: format_time(1.) }),
                job_tag: None,
                commute: Some(Commute { forward: None, backward: None }),
            },
            Activity {
                job_id: "job2".to_string(),
                activity_type: "delivery".to_string(),
                location: Some(Location::Coordinate { lat: 2., lng: 0.0 }),
                time: Some(Interval { start: format_time(2.), end: format_time(3.) }),
                job_tag: None,
                commute: Some(Commute {
                    forward: Some(CommuteInfo {
                        location: Location::Coordinate { lat: 1., lng: 0.0 },
                        distance: 10.,
                        time: Interval { start: format_time(1.), end: format_time(2.) },
                    }),
                    backward: Some(CommuteInfo {
                        location: Location::Coordinate { lat: 1., lng: 0.0 },
                        distance: 10.,
                        time: Interval { start: format_time(3.), end: format_time(4.) },
                    }),
                }),
            },
        ],
    };

    let features = get_cluster_geometry(0, 0, &stop).unwrap();

    assert_eq!(features.len(), 4);
    assert_eq!(features.iter().filter(|f| matches!(f.geometry, Geometry::Point { .. })).count(), 2);
    assert_eq!(features.iter().filter(|f| matches!(f.geometry, Geometry::LineString { .. })).count(), 2);
}
