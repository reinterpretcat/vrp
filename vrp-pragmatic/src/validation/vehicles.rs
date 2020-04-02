use super::*;
use crate::validation::common::get_time_windows;
use std::ops::Deref;
use vrp_core::models::common::TimeWindow;

/// Checks that fleet has no vehicle with duplicate type ids.
fn check_e1300_no_vehicle_types_with_duplicate_type_ids(ctx: &ValidationContext) -> Result<(), FormatError> {
    get_duplicates(ctx.vehicles().map(|vehicle| &vehicle.type_id)).map_or(Ok(()), |ids| {
        Err(FormatError::new(
            "E1300".to_string(),
            "duplicated vehicle type ids".to_string(),
            format!("remove duplicated vehicle type ids: {}", ids.join(", ")),
        ))
    })
}

/// Checks that fleet has no vehicle with duplicate ids.
fn check_e1301_no_vehicle_types_with_duplicate_ids(ctx: &ValidationContext) -> Result<(), FormatError> {
    get_duplicates(ctx.vehicles().flat_map(|vehicle| vehicle.vehicle_ids.iter())).map_or(Ok(()), |ids| {
        Err(FormatError::new(
            "E1301".to_string(),
            "duplicated vehicle ids".to_string(),
            format!("remove duplicated vehicle ids: {}", ids.join(", ")),
        ))
    })
}

/// Checks that vehicle shift time is correct.
fn check_e1302_vehicle_shift_time(ctx: &ValidationContext) -> Result<(), FormatError> {
    let type_ids = ctx
        .vehicles()
        .filter_map(|vehicle| {
            let tws = vehicle
                .shifts
                .iter()
                .map(|shift| {
                    vec![
                        shift.start.time.clone(),
                        shift.end.as_ref().map_or_else(|| shift.start.time.clone(), |end| end.time.clone()),
                    ]
                })
                .collect::<Vec<_>>();
            if check_raw_time_windows(&tws, false) {
                None
            } else {
                Some(vehicle.type_id.to_string())
            }
        })
        .collect::<Vec<_>>();

    if type_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1302".to_string(),
            "invalid start or end times in vehicle shift".to_string(),
            format!(
                "ensure that start and end time conform shift time rules, vehicle type ids: {}",
                type_ids.join(", ")
            ),
        ))
    }
}

/// Checks that break time window is correct.
fn check_e1303_vehicle_breaks_time_is_correct(ctx: &ValidationContext) -> Result<(), FormatError> {
    let type_ids = get_invalid_type_ids(
        ctx,
        Box::new(|shift, shift_time| {
            shift
                .breaks
                .as_ref()
                .map(|breaks| {
                    let tws = breaks
                        .iter()
                        .filter_map(|b| match &b.time {
                            VehicleBreakTime::TimeWindow(tw) => Some(get_time_window_from_vec(tw)),
                            _ => None,
                        })
                        .collect::<Vec<_>>();

                    check_shift_time_windows(shift_time, tws, false)
                })
                .unwrap_or(true)
        }),
    );

    if type_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1303".to_string(),
            "invalid break time windows in vehicle shift".to_string(),
            format!("ensure that break conform rules, vehicle type ids: '{}'", type_ids.join(", ")),
        ))
    }
}

/// Checks that reload time windows are correct.
fn check_e1304_vehicle_reload_time_is_correct(ctx: &ValidationContext) -> Result<(), FormatError> {
    let type_ids = get_invalid_type_ids(
        ctx,
        Box::new(|shift, shift_time| {
            shift
                .reloads
                .as_ref()
                .map(|reloads| {
                    let tws = reloads
                        .iter()
                        .filter_map(|reload| reload.times.as_ref())
                        .map(|tws| get_time_windows(tws))
                        .flatten()
                        .collect::<Vec<_>>();

                    check_shift_time_windows(shift_time, tws, true)
                })
                .unwrap_or(true)
        }),
    );

    if type_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1304".to_string(),
            "invalid reload time windows in vehicle shift".to_string(),
            format!("ensure that reload conform rules, vehicle type ids: '{}'", type_ids.join(", ")),
        ))
    }
}

fn get_invalid_type_ids(
    ctx: &ValidationContext,
    check_shift: Box<dyn Fn(&VehicleShift, Option<TimeWindow>) -> bool>,
) -> Vec<String> {
    ctx.vehicles()
        .filter_map(|vehicle| {
            let all_correct =
                vehicle.shifts.iter().all(|shift| check_shift.deref()(shift, get_shift_time_window(shift)));

            if all_correct {
                None
            } else {
                Some(vehicle.type_id.clone())
            }
        })
        .collect::<Vec<_>>()
}

fn check_shift_time_windows(
    shift_time: Option<TimeWindow>,
    tws: Vec<Option<TimeWindow>>,
    skip_intersection_check: bool,
) -> bool {
    tws.is_empty()
        || (check_time_windows(&tws, skip_intersection_check)
            && shift_time
                .as_ref()
                .map_or(true, |shift_time| tws.into_iter().map(|tw| tw.unwrap()).all(|tw| tw.intersects(shift_time))))
}

fn get_shift_time_window(shift: &VehicleShift) -> Option<TimeWindow> {
    get_time_window(
        &shift.start.time,
        &shift.end.clone().map_or_else(|| "2200-07-04T00:00:00Z".to_string(), |end| end.time),
    )
}

/// Validates vehicles from the fleet.
pub fn validate_vehicles(ctx: &ValidationContext) -> Result<(), Vec<FormatError>> {
    combine_error_results(&[
        check_e1300_no_vehicle_types_with_duplicate_type_ids(ctx),
        check_e1301_no_vehicle_types_with_duplicate_ids(ctx),
        check_e1302_vehicle_shift_time(ctx),
        check_e1303_vehicle_breaks_time_is_correct(ctx),
        check_e1304_vehicle_reload_time_is_correct(ctx),
    ])
}
