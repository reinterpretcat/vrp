#[cfg(test)]
#[path = "../../tests/unit/checker/capacity_test.rs"]
mod capacity_test;

use super::*;
use crate::utils::combine_error_results;
use std::iter::once;
use vrp_core::models::common::{Load, MultiDimLoad};

/// Checks that vehicle load is assigned correctly. The following rules are checked:
/// * max vehicle's capacity is not violated
/// * load change is correct
pub fn check_vehicle_load(context: &CheckerContext) -> Result<(), Vec<String>> {
    combine_error_results(&[check_vehicle_load_assignment(context)])
}

fn check_vehicle_load_assignment(context: &CheckerContext) -> Result<(), String> {
    context.solution.tours.iter().try_for_each(|tour| {
        let capacity = MultiDimLoad::new(context.get_vehicle(&tour.vehicle_id)?.capacity.clone());

        let legs = (0_usize..)
            .zip(tour.stops.windows(2))
            .map(|(idx, leg)| {
                (
                    idx,
                    match leg {
                        [from, to] => (from, to),
                        _ => panic!("unexpected leg configuration"),
                    },
                )
            })
            .collect::<Vec<_>>();
        let intervals: Vec<Vec<(usize, (&Stop, &Stop))>> = legs
            .iter()
            .fold(Vec::<(usize, usize)>::default(), |mut acc, (idx, (_, to))| {
                let last_idx = legs.len() - 1;
                if is_reload_stop(context, to) || *idx == last_idx {
                    let start_idx = acc.last().map_or(0_usize, |item| item.1 + 2);
                    let end_idx = if *idx == last_idx { last_idx } else { *idx - 1 };

                    acc.push((start_idx, end_idx));
                }

                acc
            })
            .into_iter()
            .map(|(start_idx, end_idx)| {
                legs.iter().cloned().skip(start_idx).take(end_idx - start_idx + 1).collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        intervals
            .iter()
            .try_fold::<_, _, Result<_, String>>(MultiDimLoad::default(), |acc, interval| {
                let (start_delivery, end_pickup) = interval
                    .iter()
                    .flat_map(|(_, (from, to))| once(from).chain(once(to)))
                    .zip(0..)
                    .filter_map(|(stop, idx)| if idx == 0 || idx % 2 == 1 { Some(stop) } else { None })
                    .flat_map(|stop| {
                        stop.activities
                            .iter()
                            .map(move |activity| (activity.clone(), context.get_activity_type(tour, stop, activity)))
                    })
                    .try_fold::<_, _, Result<_, String>>(
                        (acc, MultiDimLoad::default()),
                        |acc, (activity, activity_type)| {
                            let activity_type = activity_type?;
                            let demand = get_demand(context, &activity, &activity_type)?;
                            Ok(match demand {
                                (DemandType::StaticDelivery, demand) => (acc.0 + demand, acc.1),
                                (DemandType::StaticPickup, demand) => (acc.0, acc.1 + demand),
                                (DemandType::StaticPickupDelivery, demand) => (acc.0 + demand, acc.1 + demand),
                                _ => acc,
                            })
                        },
                    )?;

                let end_capacity = interval.iter().try_fold(start_delivery, |acc, (idx, (from, to))| {
                    let from_load = MultiDimLoad::new(from.load.clone());
                    let to_load = MultiDimLoad::new(to.load.clone());

                    if !capacity.can_fit(&from_load) || !capacity.can_fit(&to_load) {
                        return Err(format!("load exceeds capacity in tour '{}'", tour.vehicle_id));
                    }

                    let change = to.activities.iter().try_fold::<_, _, Result<_, String>>(
                        MultiDimLoad::default(),
                        |acc, activity| {
                            let activity_type = context.get_activity_type(tour, to, activity)?;
                            let (demand_type, demand) =
                                if activity.activity_type == "arrival" || activity.activity_type == "reload" {
                                    (DemandType::StaticDelivery, end_pickup)
                                } else {
                                    get_demand(context, &activity, &activity_type)?
                                };

                            Ok(match demand_type {
                                DemandType::StaticDelivery | DemandType::DynamicDelivery => acc - demand,
                                DemandType::StaticPickup | DemandType::DynamicPickup => acc + demand,
                                DemandType::None | DemandType::StaticPickupDelivery => acc,
                            })
                        },
                    )?;

                    let is_from_valid = from_load == acc;
                    let is_to_valid = to_load == from_load + change;

                    if (is_from_valid && is_to_valid) || (*idx == 0 && has_dispatch(tour)) {
                        Ok(to_load)
                    } else {
                        let message = match (is_from_valid, is_to_valid) {
                            (true, false) => format!("at stop {}", idx + 1),
                            (false, true) => format!("at stop {}", idx),
                            _ => format!("at stops {}, {}", idx, idx + 1),
                        };

                        Err(format!("load mismatch {} in tour '{}'", message, tour.vehicle_id))
                    }
                })?;

                Ok(end_capacity - end_pickup)
            })
            .map(|_| ())
    })
}

enum DemandType {
    None,
    StaticPickup,
    StaticDelivery,
    StaticPickupDelivery,
    DynamicPickup,
    DynamicDelivery,
}

fn get_demand(
    context: &CheckerContext,
    activity: &Activity,
    activity_type: &ActivityType,
) -> Result<(DemandType, MultiDimLoad), String> {
    let (is_dynamic, demand) = context.visit_job(
        activity,
        &activity_type,
        |job, task| {
            let is_dynamic = job.pickups.as_ref().map_or(false, |p| !p.is_empty())
                && job.deliveries.as_ref().map_or(false, |p| !p.is_empty());
            let demand = task.demand.clone().map_or_else(MultiDimLoad::default, MultiDimLoad::new);

            (is_dynamic, demand)
        },
        || (false, MultiDimLoad::default()),
    )?;

    let demand_type = match (is_dynamic, activity.activity_type.as_ref()) {
        (_, "replacement") => DemandType::StaticPickupDelivery,
        (true, "pickup") => DemandType::DynamicPickup,
        (true, "delivery") => DemandType::DynamicDelivery,
        (false, "pickup") => DemandType::StaticPickup,
        (false, "delivery") => DemandType::StaticDelivery,
        _ => DemandType::None,
    };

    Ok((demand_type, demand))
}

fn is_reload_stop(context: &CheckerContext, stop: &Stop) -> bool {
    context.get_stop_activity_types(stop).first().map_or(false, |a| a == "reload")
}

fn has_dispatch(tour: &Tour) -> bool {
    tour.stops
        .iter()
        .flat_map(|stop| stop.activities.iter())
        .nth(1)
        .map_or(false, |activity| activity.activity_type == "dispatch")
}
