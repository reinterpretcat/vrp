//! This module provides default strategies.

use super::*;
use crate::format::problem::*;
use crate::format::Location;
use crate::helpers::create_default_matrix_profiles;
use crate::utils::get_haversine_distance;
use crate::{format_time, parse_time};

pub const START_DAY: &str = "2020-07-04T00:00:00Z";

pub const DEFAULT_BOUNDING_BOX: ((f64, f64), (f64, f64)) = ((52.4240, 13.2148), (52.5937, 13.5970));

pub fn get_default_bounding_box_radius() -> f64 {
    let center_lat = DEFAULT_BOUNDING_BOX.0 .0 + (DEFAULT_BOUNDING_BOX.1 .0 - DEFAULT_BOUNDING_BOX.0 .0) / 2.;
    let center_lng = DEFAULT_BOUNDING_BOX.0 .1 + (DEFAULT_BOUNDING_BOX.1 .1 - DEFAULT_BOUNDING_BOX.0 .1) / 2.;

    let center = Location::new_coordinate(center_lat, center_lng);
    let corner = Location::new_coordinate(DEFAULT_BOUNDING_BOX.0 .0, DEFAULT_BOUNDING_BOX.0 .1);

    get_haversine_distance(&center, &corner)
}

pub fn default_time_plus_offset(offset: i32) -> String {
    format_time(parse_time(&START_DAY.to_string()) + from_hours(offset).as_secs_f64())
}

pub fn default_job_single_day_time_windows() -> impl Strategy<Value = Option<Vec<Vec<String>>>> {
    generate_multiple_time_windows_fixed(
        START_DAY,
        vec![from_hours(9), from_hours(14)],
        vec![from_hours(2), from_hours(4)],
        1..3,
    )
    .prop_map(|tw| Some(tw))
}

pub fn default_job_place_prototype() -> impl Strategy<Value = JobPlace> {
    job_place_prototype(
        generate_location(&DEFAULT_BOUNDING_BOX),
        generate_durations(1..10),
        default_job_single_day_time_windows(),
        generate_no_tags(),
    )
}

pub fn default_delivery_prototype() -> impl Strategy<Value = Job> {
    delivery_job_prototype(
        job_task_prototype(default_job_place_prototype(), generate_simple_demand(1..5), generate_no_order()),
        generate_no_jobs_skills(),
        generate_no_jobs_value(),
        generate_no_jobs_group(),
    )
}

pub fn default_pickup_prototype() -> impl Strategy<Value = Job> {
    pickup_job_prototype(
        job_task_prototype(default_job_place_prototype(), generate_simple_demand(1..5), generate_no_order()),
        generate_no_jobs_skills(),
        generate_no_jobs_value(),
        generate_no_jobs_group(),
    )
}

pub fn default_pickup_delivery_job_prototype() -> impl Strategy<Value = Job> {
    pickup_delivery_job_prototype(
        default_job_place_prototype(),
        default_job_place_prototype(),
        generate_simple_demand(1..4),
        generate_no_order(),
        generate_no_jobs_skills(),
        generate_no_jobs_value(),
        generate_no_jobs_group(),
    )
}

pub fn default_job_prototype() -> impl Strategy<Value = Job> {
    prop_oneof![default_delivery_prototype(), default_pickup_prototype(), default_pickup_delivery_job_prototype()]
}

pub fn default_costs_prototype() -> impl Strategy<Value = VehicleCosts> {
    from_costs(vec![
        VehicleCosts { fixed: Some(20.), distance: 0.0020, time: 0.003 },
        VehicleCosts { fixed: Some(30.), distance: 0.0015, time: 0.005 },
    ])
}

pub fn default_shift_places_prototype() -> impl Strategy<Value = (ShiftStart, Option<ShiftEnd>)> {
    generate_location(&DEFAULT_BOUNDING_BOX).prop_flat_map(|location| {
        Just((
            ShiftStart { earliest: default_time_plus_offset(9), latest: None, location: location.clone() },
            Some(ShiftEnd { earliest: None, latest: default_time_plus_offset(18), location }),
        ))
    })
}

pub fn default_breaks_prototype() -> impl Strategy<Value = Option<Vec<VehicleBreak>>> {
    Just(Some(vec![VehicleBreak {
        time: VehicleBreakTime::TimeWindow(vec![default_time_plus_offset(12), default_time_plus_offset(14)]),
        places: vec![VehicleBreakPlace { duration: 3600., location: None, tag: None }],
        policy: None,
    }]))
}

pub fn default_vehicle_profile() -> impl Strategy<Value = VehicleProfile> {
    Just(VehicleProfile { matrix: "car".to_string(), scale: None })
}

pub fn default_matrix_profiles() -> impl Strategy<Value = Vec<MatrixProfile>> {
    Just(create_default_matrix_profiles())
}

pub fn default_vehicle_shifts() -> impl Strategy<Value = Vec<VehicleShift>> {
    generate_shifts(
        generate_shift(
            default_shift_places_prototype(),
            generate_no_dispatch(),
            default_breaks_prototype(),
            generate_no_reloads(),
        ),
        1..2,
    )
}

pub fn default_vehicle_type_prototype() -> impl Strategy<Value = VehicleType> {
    generate_vehicle(
        2..4,
        Just(VehicleProfile { matrix: "car".to_string(), scale: None }),
        generate_simple_capacity(30..50),
        default_costs_prototype(),
        generate_no_vehicle_skills(),
        generate_no_limits(),
        default_vehicle_shifts(),
    )
}
