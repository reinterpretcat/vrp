#[cfg(test)]
#[path = "../../tests/unit/checker/capacity_test.rs"]
mod capacity_test;

use super::*;
use crate::utils::combine_error_results;
use std::iter::once;
use vrp_core::models::common::{Load, MultiDimLoad};
use vrp_core::prelude::GenericResult;

/// Checks that vehicle load is assigned correctly. The following rules are checked:
/// * max vehicle's capacity is not violated
/// * load change is correct
pub fn check_vehicle_load(context: &CheckerContext) -> Result<(), Vec<GenericError>> {
    combine_error_results(&[check_vehicle_load_assignment(context), check_resource_consumption(context)])
}

fn check_vehicle_load_assignment(context: &CheckerContext) -> GenericResult<()> {
    context.solution.tours.iter().try_for_each::<_, GenericResult<_>>(|tour| {
        let capacity = MultiDimLoad::new(context.get_vehicle(&tour.vehicle_id)?.capacity.clone());
        let intervals = get_intervals(context, tour);

        intervals
            .iter()
            .try_fold::<_, _, GenericResult<_>>(MultiDimLoad::default(), |acc, interval| {
                let (start_delivery, end_pickup) = get_activities_from_interval(context, tour, interval.as_slice())
                    .try_fold::<_, _, GenericResult<_>>(
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

                let end_capacity =
                    interval.iter().try_fold::<_, _, GenericResult<_>>(start_delivery, |acc, (idx, (from, to))| {
                        let from_load = MultiDimLoad::new(from.load().clone());
                        let to_load = MultiDimLoad::new(to.load().clone());

                        if !capacity.can_fit(&from_load) || !capacity.can_fit(&to_load) {
                            return Err(format!("load exceeds capacity in tour '{}'", tour.vehicle_id).into());
                        }

                        let change = to.activities().iter().try_fold::<_, _, GenericResult<_>>(
                            MultiDimLoad::default(),
                            |acc, activity| {
                                let activity_type = context.get_activity_type(tour, to, activity)?;
                                let (demand_type, demand) =
                                    if activity.activity_type == "arrival" || activity.activity_type == "reload" {
                                        (DemandType::StaticDelivery, end_pickup)
                                    } else {
                                        get_demand(context, activity, &activity_type)?
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

                        if is_from_valid && is_to_valid {
                            Ok(to_load)
                        } else {
                            let message = match (is_from_valid, is_to_valid) {
                                (true, false) => format!("at stop {}", idx + 1),
                                (false, true) => format!("at stop {idx}"),
                                _ => format!("at stops {}, {}", idx, idx + 1),
                            };

                            Err(format!("load mismatch {} in tour '{}'", message, tour.vehicle_id).into())
                        }
                    })?;

                Ok(end_capacity - end_pickup)
            })
            .map(|_| ())
    })
}

fn check_resource_consumption(context: &CheckerContext) -> GenericResult<()> {
    let resources = context
        .problem
        .fleet
        .resources
        .iter()
        .flat_map(|resources| resources.iter().cloned())
        .map(|resource| match resource {
            VehicleResource::Reload { id, capacity } => (id, MultiDimLoad::new(capacity)),
        })
        .collect::<HashMap<_, _>>();

    let consumption: HashMap<String, MultiDimLoad> = context
        .solution
        .tours
        .iter()
        .flat_map(|tour| {
            get_intervals(context, tour).into_iter().filter_map(|interval| {
                let resource_id = interval.first().and_then(|(_, (start, _))| {
                    start
                        .activities()
                        .iter()
                        .filter_map(|activity| context.get_activity_type(tour, start, activity).ok())
                        .filter_map(|activity| match activity {
                            ActivityType::Reload(reload) => Some(reload),
                            _ => None,
                        })
                        .filter_map(|reload| reload.resource_id.as_ref().cloned())
                        .next()
                });

                if let Some(resource_id) = resource_id {
                    let consumption = get_activities_from_interval(context, tour, interval.as_slice())
                        .filter_map(|(activity, activity_type)| Some(activity).zip(activity_type.ok()))
                        .filter_map(|(activity, activity_type)| get_demand(context, &activity, &activity_type).ok())
                        .filter_map(|(demand_type, demand_value)| match demand_type {
                            DemandType::StaticDelivery => Some(demand_value),
                            _ => None,
                        })
                        .fold(MultiDimLoad::default(), |acc, demand| acc + demand);
                    Some((resource_id, consumption))
                } else {
                    None
                }
            })
        })
        .fold(HashMap::default(), |mut acc, (resource_id, consumption)| {
            let entry = acc.entry(resource_id).or_default();
            *entry = *entry + consumption;

            acc
        });

    consumption.into_iter().try_for_each(|(resource_id, consumed)| {
        let available = *resources.get(&resource_id).ok_or_else(|| {
            GenericError::from(format!("cannot find resource '{resource_id}' in list of available resources"))
        })?;

        if consumed > available {
            Err(GenericError::from(format!(
                "consumed more resource '{resource_id}' than available: {consumed} vs {available}"
            )))
        } else {
            Ok(())
        }
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
) -> GenericResult<(DemandType, MultiDimLoad)> {
    let (is_dynamic, demand) = context.visit_job(
        activity,
        activity_type,
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

fn get_intervals<'a>(context: &CheckerContext, tour: &'a Tour) -> Vec<Vec<(usize, (&'a Stop, &'a Stop))>> {
    let legs = tour
        .stops
        .windows(2)
        .enumerate()
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

    legs.iter()
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
        .collect()
}

fn get_activities_from_interval<'a>(
    context: &'a CheckerContext,
    tour: &'a Tour,
    interval: &'a [(usize, (&Stop, &Stop))],
) -> impl Iterator<Item = (Activity, GenericResult<ActivityType>)> + 'a {
    interval
        .iter()
        .flat_map(|(_, (from, to))| once(from).chain(once(to)))
        .enumerate()
        .filter_map(|(idx, stop)| if idx == 0 || idx % 2 == 1 { Some(stop) } else { None })
        .flat_map(move |stop| {
            stop.activities()
                .iter()
                .map(move |activity| (activity.clone(), context.get_activity_type(tour, stop, activity)))
        })
}

fn is_reload_stop(context: &CheckerContext, stop: &Stop) -> bool {
    context.get_stop_activity_types(stop).first().map_or(false, |a| a == "reload")
}
