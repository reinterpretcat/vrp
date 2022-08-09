#[cfg(test)]
#[path = "../../tests/unit/validation/vehicles_test.rs"]
mod vehicles_test;

use super::*;
use crate::utils::combine_error_results;
use crate::validation::common::get_time_windows;
use crate::{parse_time, parse_time_safe};
use hashbrown::HashSet;
use std::cmp::Ordering;
use std::ops::Deref;
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
                            VehicleBreak::Required { time: VehicleRequiredBreakTime::ExactTime(time), duration } => {
                                Some(parse_time_safe(time).ok().map(|start| TimeWindow::new(start, start + *duration)))
                            }
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
fn check_e1305_vehicle_limit_area_is_correct(ctx: &ValidationContext) -> Result<(), FormatError> {
    let area_index = ctx
        .problem
        .plan
        .areas
        .iter()
        .flat_map(|areas| areas.iter().map(|area| (&area.id, &area.jobs)))
        .collect::<HashMap<_, _>>();

    let type_ids = ctx
        .vehicles()
        .filter(|vehicle| {
            let area_ids = vehicle
                .limits
                .as_ref()
                .and_then(|l| l.areas.as_ref())
                .iter()
                .flat_map(|areas| areas.iter())
                .flat_map(|areas| areas.iter())
                .collect::<Vec<_>>();

            // check area presence
            if !area_ids.iter().all(|limit| area_index.get(&limit.area_id).is_some()) {
                return true;
            }

            let all_jobs = area_ids
                .iter()
                .flat_map(|limit| area_index.get(&limit.area_id).iter().cloned().collect::<Vec<_>>().into_iter())
                .flat_map(|job_ids| job_ids.iter())
                .collect::<Vec<_>>();

            // check job presence
            if !all_jobs.iter().all(|&job_id| ctx.job_index.contains_key(job_id)) {
                return true;
            }

            // check job uniqueness
            let unique_jobs = all_jobs.iter().collect::<HashSet<_>>();
            all_jobs.len() != unique_jobs.len()
        })
        .map(|vehicle| vehicle.type_id.to_string())
        .collect::<Vec<_>>();

    if type_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1305".to_string(),
            "invalid allowed area definition in vehicle limits".to_string(),
            format!(
                "ensure that areas for the same vehicle contains unique and valid job ids, \
                 vehicle type ids: '{}'",
                type_ids.join(", ")
            ),
        ))
    }
}

fn check_e1306_vehicle_dispatch_is_correct(ctx: &ValidationContext) -> Result<(), FormatError> {
    let type_ids = get_invalid_type_ids(
        ctx,
        Box::new(move |vehicle, shift, shift_time| {
            shift.dispatch.as_ref().map_or(true, |dispatch| {
                let has_valid_tw = dispatch.iter().flat_map(|dispatch| dispatch.limits.iter()).all(|limit| {
                    let start = parse_time(&limit.start);
                    let end = parse_time(&limit.end);

                    compare_floats(start, end) != Ordering::Greater
                        && shift_time.as_ref().map_or(true, |tw| {
                            TimeWindow::new(start, start).intersects(tw) && TimeWindow::new(end, end).intersects(tw)
                        })
                });

                let has_valid_max = dispatch.iter().all(|dispatch| {
                    dispatch.limits.iter().map(|limit| limit.max).sum::<usize>() == vehicle.vehicle_ids.len()
                });

                has_valid_tw
                    && has_valid_max
                    && dispatch.iter().map(|dispatch| dispatch.location.clone()).collect::<HashSet<_>>().len()
                        == dispatch.len()
            })
        }),
    );

    if type_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1306".to_string(),
            "invalid dispatch in vehicle shift".to_string(),
            format!(
                "ensure that all dispatch have proper dispatch parameters and unique locations. Vehicle type ids: '{}'",
                type_ids.join(", ")
            ),
        ))
    }
}

/// Checks that vehicle area restrictions are valid.
fn check_e1307_vehicle_has_no_zero_costs(ctx: &ValidationContext) -> Result<(), FormatError> {
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
            "E1307".to_string(),
            "time and duration costs are zeros".to_string(),
            format!(
                "ensure that either time or distance cost is non-zero, \
                 vehicle type ids: '{}'",
                type_ids.join(", ")
            ),
        ))
    }
}

fn check_e1308_vehicle_required_break_rescheduling(ctx: &ValidationContext) -> Result<(), FormatError> {
    let type_ids = get_invalid_type_ids(
        ctx,
        Box::new(|_, shift, _| {
            shift
                .breaks
                .as_ref()
                .map(|breaks| {
                    let has_required_break = breaks.iter().any(|br| matches!(br, VehicleBreak::Required { .. }));
                    let has_rescheduling =
                        shift.start.latest.as_ref().map_or(true, |latest| *latest != shift.start.earliest);

                    !(has_required_break && has_rescheduling)
                })
                .unwrap_or(true)
        }),
    );

    if type_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1308".to_string(),
            "required break is used with departure rescheduling".to_string(),
            format!("when required break is used, start.latest should be set equal to start.earliest in the shift, check vehicle type ids: '{}'", type_ids.join(", ")),
        ))
    }
}

fn check_e1309_vehicle_reload_resources(ctx: &ValidationContext) -> Result<(), FormatError> {
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
            "E1309".to_string(),
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
            "E1309".to_string(),
            "invalid vehicle reload resource".to_string(),
            format!(
                "make sure that fleet has all reload resources defined, check vehicle type ids: '{}'",
                type_ids.join(", ")
            ),
        ))
    }
}

fn get_invalid_type_ids(
    ctx: &ValidationContext,
    check_shift: Box<dyn Fn(&VehicleType, &VehicleShift, Option<TimeWindow>) -> bool>,
) -> Vec<String> {
    ctx.vehicles()
        .filter_map(|vehicle| {
            let all_correct =
                vehicle.shifts.iter().all(|shift| check_shift.deref()(vehicle, shift, get_shift_time_window(shift)));

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
pub fn validate_vehicles(ctx: &ValidationContext) -> Result<(), Vec<FormatError>> {
    combine_error_results(&[
        check_e1300_no_vehicle_types_with_duplicate_type_ids(ctx),
        check_e1301_no_vehicle_types_with_duplicate_ids(ctx),
        check_e1302_vehicle_shift_time(ctx),
        check_e1303_vehicle_breaks_time_is_correct(ctx),
        check_e1304_vehicle_reload_time_is_correct(ctx),
        check_e1305_vehicle_limit_area_is_correct(ctx),
        check_e1306_vehicle_dispatch_is_correct(ctx),
        check_e1307_vehicle_has_no_zero_costs(ctx),
        check_e1308_vehicle_required_break_rescheduling(ctx),
        check_e1309_vehicle_reload_resources(ctx),
    ])
}
