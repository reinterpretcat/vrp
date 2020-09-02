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

parameterized_test! {can_detect_invalid_depots, (allowed_areas, expected), {
    can_detect_invalid_depots_impl(allowed_areas, expected);
}}

can_detect_invalid_depots! {
    case01: (&[(0., None)], Some("E1306".to_string())),
    case02: (&[(1., None)], None),
    case03: (&[(1., None), (1., None)], Some("E1306".to_string())),
    case04: (&[(1., None), (2., None)], None),
    case05: (&[(0., None), (1., None)], Some("E1306".to_string())),

    case06: (&[(1., Some(vec![vec![format_time(0.), format_time(100.)]]))], None),
    case07: (&[(1., Some(vec![vec![format_time(1001.), format_time(1010.)]]))], Some("E1306".to_string())),
    case08: (&[(1., Some(vec![vec![format_time(100.), format_time(10.)]]))], Some("E1306".to_string())),
}

fn can_detect_invalid_depots_impl(depots: &[(f64, Option<Vec<Vec<String>>>)], expected: Option<String>) {
    let depots = Some(
        depots
            .into_iter()
            .cloned()
            .map(|(lat, times)| VehicleCargoPlace {
                location: Location::Coordinate { lat, lng: 0. },
                duration: 1.,
                times,
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
