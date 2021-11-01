use super::*;
use crate::format::problem::Problem as FormatProblem;
use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_create_geo_json_from_solution() {
    let problem = FormatProblem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![1., 0.]),
                create_delivery_job_with_demand("job2", vec![2., 0.], vec![10]),
                create_delivery_job("job3", vec![3., 0.]),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_default_vehicle("my_vehicle")],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);
    let core_problem = (problem.clone(), vec![matrix.clone()]).read_pragmatic().unwrap();
    let solution = solve_with_cheapest_insertion(problem, Some(vec![matrix]));
    let geo_json = create_geojson_solution(&core_problem, &solution).unwrap();

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
