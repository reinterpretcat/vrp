use super::common::get_duplicates;
use super::*;

/// Checks that fleet has no vehicle with duplicate type ids.
fn check_e1003_no_vehicle_types_with_duplicate_type_ids(ctx: &ValidationContext) -> Result<(), String> {
    get_duplicates(ctx.vehicles().map(|vehicle| &vehicle.type_id))
        .map_or(Ok(()), |ids| Err(format!("E1003: Duplicated vehicle type ids: {}", ids.join(", "))))
}

/// Checks that fleet has no vehicle with duplicate ids.
fn check_e1004_no_vehicle_types_with_duplicate_ids(ctx: &ValidationContext) -> Result<(), String> {
    get_duplicates(ctx.vehicles().flat_map(|vehicle| vehicle.vehicle_ids.iter()))
        .map_or(Ok(()), |ids| Err(format!("E1004: Duplicated vehicle ids: {}", ids.join(", "))))
}

/// Checks that vehicle shift time is correct.
fn check_e1005_vehicle_shift_time(ctx: &ValidationContext) -> Result<(), String> {
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
            if check_time_windows(&tws) {
                None
            } else {
                Some(vehicle.type_id.to_string())
            }
        })
        .collect::<Vec<_>>();

    if type_ids.is_empty() {
        Ok(())
    } else {
        Err(format!("E1005: Invalid time windows in vehicle shifts: {}", type_ids.join(", ")))
    }
}

/// Validates vehicles from the fleet.
pub fn validate_vehicles(ctx: &ValidationContext) -> Result<(), Vec<String>> {
    let errors = check_e1003_no_vehicle_types_with_duplicate_type_ids(ctx)
        .err()
        .iter()
        .cloned()
        .chain(check_e1004_no_vehicle_types_with_duplicate_ids(ctx).err().iter().cloned())
        .chain(check_e1005_vehicle_shift_time(ctx).err().iter().cloned())
        .collect::<Vec<_>>();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
