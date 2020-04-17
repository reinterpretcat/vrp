use super::*;
use crate::format::Location;
use crate::helpers::*;

fn coord(lat: f64, lng: f64) -> Location {
    Location { lat, lng }
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

fn can_detect_invalid_area_impl(allowed_areas: Option<Vec<Vec<Location>>>, expected: Option<()>) {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                limits: Some(VehicleLimits { max_distance: None, shift_time: None, allowed_areas }),
                ..create_default_vehicle_type()
            }],
            profiles: vec![],
        },
        ..create_empty_problem()
    };

    let result = check_e1305_vehicle_limit_area_is_correct(&ValidationContext::new(&problem, None));

    assert_eq!(result.err().map(|err| err.code), expected.map(|_| "E1305".to_string()));
}
