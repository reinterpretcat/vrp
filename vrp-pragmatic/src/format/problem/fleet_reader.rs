#[cfg(test)]
#[path = "../../../tests/unit/format/problem/fleet_reader_test.rs"]
mod fleet_reader_test;

use crate::extensions::create_typed_actor_groups;
use crate::format::coord_index::CoordIndex;
use crate::format::problem::reader::{ApiProblem, ProblemProperties};
use crate::format::problem::Matrix;
use crate::parse_time;
use hashbrown::{HashMap, HashSet};
use std::sync::Arc;
use vrp_core::construction::constraints::extensions::{NoTravelLimits, SimpleTravelLimits};
use vrp_core::models::common::*;
use vrp_core::models::problem::*;

pub(crate) fn get_profile_index_map(api_problem: &ApiProblem) -> HashMap<String, usize> {
    api_problem.fleet.profiles.iter().fold(Default::default(), |mut acc, profile| {
        if acc.get(&profile.name).is_none() {
            acc.insert(profile.name.clone(), acc.len());
        }
        acc
    })
}

pub(crate) fn create_transport_costs(
    api_problem: &ApiProblem,
    matrices: &[Matrix],
) -> Result<Arc<dyn TransportCost + Sync + Send>, String> {
    if !matrices.iter().all(|m| m.profile.is_some()) && !matrices.iter().all(|m| m.profile.is_none()) {
        return Err("all matrices should have profile set or none of them".to_string());
    }

    if matrices.iter().any(|m| m.profile.is_none()) && matrices.iter().any(|m| m.timestamp.is_some()) {
        return Err("when timestamp is set, all matrices should have profile set".to_string());
    }

    let matrix_profiles = get_profile_index_map(api_problem);
    if matrix_profiles.len() > matrices.len() {
        return Err(format!(
            "not enough routing matrices specified for fleet profiles defined: \
             {} must be less or equal to {}",
            matrix_profiles.len(),
            matrices.len()
        ));
    }

    let travel_limits = read_travel_limits(api_problem);

    let matrix_data = matrices
        .iter()
        .enumerate()
        .map(|(idx, matrix)| {
            let profile = matrix.profile.as_ref().and_then(|p| matrix_profiles.get(p)).cloned().unwrap_or(idx);
            (profile, matrix.timestamp.clone(), matrix)
        })
        .map(|(profile, timestamp, matrix)| {
            let (durations, distances) = if let Some(error_codes) = &matrix.error_codes {
                let capacity = matrix.distances.len();

                let mut durations: Vec<Duration> = Vec::with_capacity(capacity);
                let mut distances: Vec<Distance> = Vec::with_capacity(capacity);
                for (i, error) in error_codes.iter().enumerate() {
                    if *error > 0 {
                        durations.push(-1.);
                        distances.push(-1.);
                    } else {
                        durations.push(*matrix.travel_times.get(i).unwrap() as f64);
                        distances.push(*matrix.distances.get(i).unwrap() as f64);
                    }
                }
                (durations, distances)
            } else {
                (
                    matrix.travel_times.iter().map(|d| *d as f64).collect(),
                    matrix.distances.iter().map(|d| *d as f64).collect(),
                )
            };

            MatrixData::new(profile, timestamp.map(|t| parse_time(&t)), durations, distances)
        })
        .collect::<Vec<_>>();

    let matrix_indices = matrix_data.iter().map(|data| data.index).collect::<HashSet<_>>().len();
    if matrix_profiles.len() != matrix_indices {
        return Err("amount of fleet profiles does not match matrix profiles".to_string());
    }

    create_matrix_transport_cost(matrix_data, travel_limits)
}

pub(crate) fn read_fleet(api_problem: &ApiProblem, props: &ProblemProperties, coord_index: &CoordIndex) -> Fleet {
    let profile_indices = get_profile_index_map(api_problem);
    let area_index = api_problem
        .plan
        .areas
        .iter()
        .flat_map(|areas| areas.iter().map(|area| (&area.id, area)))
        .collect::<HashMap<_, _>>();
    let mut vehicles: Vec<Arc<Vehicle>> = Default::default();

    api_problem.fleet.vehicles.iter().for_each(|vehicle| {
        let costs = Costs {
            fixed: vehicle.costs.fixed.unwrap_or(0.),
            per_distance: vehicle.costs.distance,
            per_driving_time: vehicle.costs.time,
            per_waiting_time: vehicle.costs.time,
            per_service_time: vehicle.costs.time,
        };

        let index = *profile_indices.get(&vehicle.profile.matrix).unwrap();
        let profile = Profile::new(index, vehicle.profile.scale);

        let tour_size = vehicle.limits.as_ref().and_then(|l| l.tour_size);
        let mut area_jobs = vehicle.limits.as_ref().and_then(|l| l.areas.as_ref()).map({
            let area_index = &area_index;
            move |areas| {
                areas
                    .iter()
                    .enumerate()
                    .flat_map(move |(order, area_ids)| {
                        area_ids.iter().flat_map(move |limit| {
                            area_index
                                .get(&limit.area_id)
                                .iter()
                                .flat_map(|&&area| {
                                    area.jobs.iter().map(|job_id| (job_id.clone(), (order, limit.job_value)))
                                })
                                .collect::<Vec<_>>()
                                .into_iter()
                        })
                    })
                    .collect::<HashMap<_, _>>()
            }
        });

        for (shift_index, shift) in vehicle.shifts.iter().enumerate() {
            let start = {
                let location = coord_index.get_by_loc(&shift.start.location).unwrap();
                let earliest = parse_time(&shift.start.earliest);
                let latest = shift.start.latest.as_ref().map(|time| parse_time(time));
                (location, earliest, latest)
            };

            let end = shift.end.as_ref().map(|end| {
                let location = coord_index.get_by_loc(&end.location).unwrap();
                let time = parse_time(&end.latest);
                (location, time)
            });

            let details = vec![VehicleDetail {
                start: Some(VehiclePlace {
                    location: start.0,
                    time: TimeInterval { earliest: Some(start.1), latest: start.2 },
                }),
                end: end.map(|(location, time)| VehiclePlace {
                    location,
                    time: TimeInterval { earliest: None, latest: Some(time) },
                }),
            }];

            vehicle.vehicle_ids.iter().for_each(|vehicle_id| {
                let mut dimens: Dimensions = Default::default();
                dimens.set_value("type_id", vehicle.type_id.clone());
                dimens.set_value("shift_index", shift_index);
                dimens.set_id(vehicle_id);

                if let Some(area_jobs) = area_jobs.take() {
                    dimens.set_value("areas", area_jobs);
                }

                if let Some(tour_size) = tour_size {
                    dimens.set_value("tour_size", tour_size);
                }

                if props.has_multi_dimen_capacity {
                    dimens.set_capacity(MultiDimLoad::new(vehicle.capacity.clone()));
                } else {
                    dimens.set_capacity(SingleDimLoad::new(*vehicle.capacity.first().unwrap()));
                }
                add_vehicle_skills(&mut dimens, &vehicle.skills);

                vehicles.push(Arc::new(Vehicle {
                    profile: profile.clone(),
                    costs: costs.clone(),
                    dimens,
                    details: details.clone(),
                }));
            });
        }
    });

    let drivers = vec![Arc::new(Driver {
        costs: Costs {
            fixed: 0.0,
            per_distance: 0.0,
            per_driving_time: 0.0,
            per_waiting_time: 0.0,
            per_service_time: 0.0,
        },
        dimens: Default::default(),
        details: vec![],
    })];

    Fleet::new(drivers, vehicles, Box::new(|actors| create_typed_actor_groups(actors)))
}

fn read_travel_limits(api_problem: &ApiProblem) -> Arc<dyn TravelLimits + Send + Sync> {
    let (duration, distance) = api_problem
        .fleet
        .vehicles
        .iter()
        .filter_map(|vehicle| vehicle.limits.as_ref().map(|limits| (vehicle, limits)))
        .fold((HashMap::new(), HashMap::new()), |(mut duration, mut distance), (vehicle, limits)| {
            limits.max_distance.iter().for_each(|max_distance| {
                distance.insert(vehicle.type_id.clone(), *max_distance);
            });

            limits.shift_time.iter().for_each(|shift_time| {
                duration.insert(vehicle.type_id.clone(), *shift_time);
            });

            (duration, distance)
        });

    if duration.is_empty() && distance.is_empty() {
        Arc::new(NoTravelLimits::default())
    } else {
        Arc::new(SimpleTravelLimits::new(
            Arc::new(move |actor: &Actor| {
                distance.get(actor.vehicle.dimens.get_value::<String>("type_id").unwrap()).cloned()
            }),
            Arc::new(move |actor: &Actor| {
                duration.get(actor.vehicle.dimens.get_value::<String>("type_id").unwrap()).cloned()
            }),
        ))
    }
}

fn add_vehicle_skills(dimens: &mut Dimensions, skills: &Option<Vec<String>>) {
    if let Some(skills) = skills {
        dimens.set_value("skills", skills.iter().cloned().collect::<HashSet<_>>());
    }
}
