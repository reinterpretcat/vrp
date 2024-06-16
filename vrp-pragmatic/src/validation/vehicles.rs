#[cfg(test)]
#[path = "../../tests/unit/validation/vehicles_test.rs"]
mod vehicles_test;

use super::*;
use crate::utils::combine_error_results;
use crate::validation::common::get_time_windows;
use crate::{parse_time, parse_time_safe};
use std::cmp::Ordering;
use std::collections::HashSet;
use vrp_core::models::common::TimeWindow;
use vrp_core::utils::compare_floats;

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
                        shift.start.earliest.clone(),
                        shift.end.as_ref().map_or_else(|| shift.start.earliest.clone(), |end| end.latest.clone()),
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
        Box::new(|_, shift, shift_time| {
            shift
                .breaks
                .as_ref()
                .map(|breaks| {
                    let tws = breaks
                        .iter()
                        .filter_map(|b| match b {
                            VehicleBreak::Optional { time: VehicleOptionalBreakTime::TimeWindow(tw), .. } => {
                                Some(get_time_window_from_vec(tw))
                            }
                            VehicleBreak::Required {
                                time: VehicleRequiredBreakTime::OffsetTime { earliest, latest },
                                duration,
                            } => {
                                let departure = parse_time(&shift.start.earliest);
                                Some(Some(TimeWindow::new(departure + *earliest, departure + *latest + *duration)))
                            }
                            VehicleBreak::Required {
                                time: VehicleRequiredBreakTime::ExactTime { earliest, latest },
                                duration,
                            } => Some(
                                parse_time_safe(earliest)
                                    .ok()
                                    .zip(parse_time_safe(latest).ok())
                                    .map(|(start, end)| TimeWindow::new(start, end + *duration)),
                            ),
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
        Box::new(|_, shift, shift_time| {
            shift
                .reloads
                .as_ref()
                .map(|reloads| {
                    let tws = reloads
                        .iter()
                        .filter_map(|reload| reload.times.as_ref())
                        .flat_map(|tws| get_time_windows(tws))
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

/// Checks that vehicle area restrictions are valid.
fn check_e1306_vehicle_has_no_zero_costs(ctx: &ValidationContext) -> Result<(), FormatError> {
    let type_ids = ctx
        .vehicles()
        .filter(|vehicle| {
            compare_floats(vehicle.costs.time, 0.) == Ordering::Equal
                && compare_floats(vehicle.costs.distance, 0.) == Ordering::Equal
        })
        .map(|vehicle| vehicle.type_id.to_string())
        .collect::<Vec<_>>();

    if type_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1306".to_string(),
            "time and duration costs are zeros".to_string(),
            format!(
                "ensure that either time or distance cost is non-zero, \
                 vehicle type ids: '{}'",
                type_ids.join(", ")
            ),
        ))
    }
}

fn check_e1307_vehicle_offset_break_rescheduling(ctx: &ValidationContext) -> Result<(), FormatError> {
    let type_ids = get_invalid_type_ids(
        ctx,
        Box::new(|_, shift, _| {
            shift
                .breaks
                .as_ref()
                .map(|breaks| {
                    let has_time_offset = breaks.iter().any(|br| {
                        matches!(
                            br,
                            VehicleBreak::Required { time: VehicleRequiredBreakTime::OffsetTime { .. }, .. }
                                | VehicleBreak::Optional { time: VehicleOptionalBreakTime::TimeOffset { .. }, .. }
                        )
                    });
                    let has_rescheduling =
                        shift.start.latest.as_ref().map_or(true, |latest| *latest != shift.start.earliest);

                    !(has_time_offset && has_rescheduling)
                })
                .unwrap_or(true)
        }),
    );

    if type_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1307".to_string(),
            "time offset interval for break is used with departure rescheduling".to_string(),
            format!("when time offset is used, start.latest should be set equal to start.earliest in the shift, check vehicle type ids: '{}'", type_ids.join(", ")),
        ))
    }
}

fn check_e1308_vehicle_reload_resources(ctx: &ValidationContext) -> Result<(), FormatError> {
    let reload_resource_ids = ctx
        .problem
        .fleet
        .resources
        .iter()
        .flat_map(|resources| resources.iter())
        .map(|resource| match resource {
            VehicleResource::Reload { id, .. } => id.to_string(),
        })
        .collect::<Vec<_>>();

    let unique_resource_ids = reload_resource_ids.iter().cloned().collect::<HashSet<_>>();

    if reload_resource_ids.len() != unique_resource_ids.len() {
        return Err(FormatError::new(
            "E1308".to_string(),
            "invalid vehicle reload resource".to_string(),
            "make sure that fleet reload resource ids are unique".to_string(),
        ));
    }

    let type_ids = get_invalid_type_ids(
        ctx,
        Box::new(move |_, shift, _| {
            shift
                .reloads
                .as_ref()
                .iter()
                .flat_map(|reloads| reloads.iter())
                .filter_map(|reload| reload.resource_id.as_ref())
                .all(|resource_id| unique_resource_ids.contains(resource_id))
        }),
    );

    if type_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1308".to_string(),
            "invalid vehicle reload resource".to_string(),
            format!(
                "make sure that fleet has all reload resources defined, check vehicle type ids: '{}'",
                type_ids.join(", ")
            ),
        ))
    }
}

type CheckShiftFn = Box<dyn Fn(&VehicleType, &VehicleShift, Option<TimeWindow>) -> bool>;

fn get_invalid_type_ids(ctx: &ValidationContext, check_shift_fn: CheckShiftFn) -> Vec<String> {
    ctx.vehicles()
        .filter_map(|vehicle| {
            let all_correct =
                vehicle.shifts.iter().all(|shift| (check_shift_fn)(vehicle, shift, get_shift_time_window(shift)));

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
        &shift.start.earliest,
        &shift.end.clone().map_or_else(|| "2200-07-04T00:00:00Z".to_string(), |end| end.latest),
    )
}

/// Validates vehicles from the fleet.
pub fn validate_vehicles(ctx: &ValidationContext) -> Result<(), MultiFormatError> {
    combine_error_results(&[
        check_e1300_no_vehicle_types_with_duplicate_type_ids(ctx),
        check_e1301_no_vehicle_types_with_duplicate_ids(ctx),
        check_e1302_vehicle_shift_time(ctx),
        check_e1303_vehicle_breaks_time_is_correct(ctx),
        check_e1304_vehicle_reload_time_is_correct(ctx),
        check_e1306_vehicle_has_no_zero_costs(ctx),
        check_e1307_vehicle_offset_break_rescheduling(ctx),
        check_e1308_vehicle_reload_resources(ctx),
    ])
    .map_err(From::from)
}
