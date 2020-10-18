use super::*;
use crate::format::Location;
use crate::format_time;
use crate::helpers::*;

fn coord(lat: f64, lng: f64) -> Location {
    Location::Coordinate { lat, lng }
}

parameterized_test! {can_detect_invalid_area, (allowed_areas, expected), {
    can_detect_invalid_area_impl(allowed_areas, expected);
}}

can_detect_invalid_area! {
    case01: (None, None),
    case02: (Some(vec![vec![coord(0., 0.), coord(0., 1.), coord(1., 1.)]]), None),
    case03: (Some(vec![vec![coord(0., 0.), coord(0., 1.), coord(1., 1.), coord(1., 0.)]]), None),

    case04: (Some(vec![]), Some(())),
    case05: (Some(vec![vec![]]), Some(())),
    case06: (Some(vec![vec![coord(0., 0.)]]), Some(())),
    case07: (Some(vec![vec![coord(0., 0.), coord(0., 1.)]]), Some(())),
    case08: (Some(vec![vec![coord(0., 0.), coord(0., 1.), coord(1., 1.)], vec![coord(0., 1.)]]), Some(())),
}

fn can_detect_invalid_area_impl(allowed_shapes: Option<Vec<Vec<Location>>>, expected: Option<()>) {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                limits: Some(VehicleLimits {
                    max_distance: None,
                    shift_time: None,
                    allowed_areas: allowed_shapes.map(|shapes| {
                        shapes.into_iter().map(|shape| AreaLimit { priority: None, outer_shape: shape }).collect()
                    }),
                }),
                ..create_default_vehicle_type()
            }],
            profiles: vec![],
        },
        ..create_empty_problem()
    };

    let result = check_e1305_vehicle_limit_area_is_correct(&ValidationContext::new(&problem, None));

    assert_eq!(result.err().map(|err| err.code), expected.map(|_| "E1305".to_string()));
}

parameterized_test! {can_detect_invalid_depots, (allowed_areas, expected), {
    can_detect_invalid_depots_impl(allowed_areas, expected);
}}

can_detect_invalid_depots! {
    case01: (&[(0., (0., 10.))], None),
    case02: (&[(1., (0., 10.))], None),
    case03: (&[(1., (0., 10.)), (1., (0., 10.))], Some("E1306".to_string())),
    case04: (&[(1., (0., 10.)), (2., (0., 10.))], None),

    case05: (&[(1., (0., 10.))], None),
    case06: (&[(1., (1001., 1010.))], Some("E1306".to_string())),
    case07: (&[(1., (10., 1.))], Some("E1306".to_string())),
}

fn can_detect_invalid_depots_impl(depots: &[(f64, (f64, f64))], expected: Option<String>) {
    let depots = Some(
        depots
            .into_iter()
            .cloned()
            .map(|(lat, times)| VehicleDepot {
                location: Location::Coordinate { lat, lng: 0. },
                dispatch: vec![VehicleDispatch { max: 1, start: format_time(times.0), end: format_time(times.1) }],
                tag: None,
            })
            .collect(),
    );
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift { depots, ..create_default_vehicle_shift() }],
                ..create_default_vehicle_type()
            }],
            profiles: vec![],
        },
        ..create_empty_problem()
    };

    let result = check_e1306_vehicle_depot_is_correct(&ValidationContext::new(&problem, None));

    assert_eq!(result.err().map(|err| err.code), expected);
}
