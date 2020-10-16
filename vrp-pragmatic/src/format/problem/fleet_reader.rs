#[cfg(test)]
#[path = "../../../tests/unit/format/problem/fleet_reader_test.rs"]
mod fleet_reader_test;

use crate::extensions::create_typed_actor_groups;
use crate::format::coord_index::CoordIndex;
use crate::format::problem::reader::{ApiProblem, ProblemProperties};
use crate::format::problem::Matrix;
use crate::parse_time;
use hashbrown::{HashMap, HashSet};
use std::iter::FromIterator;
use std::sync::Arc;
use vrp_core::construction::constraints::{Area, TravelLimitFunc};
use vrp_core::models::common::*;
use vrp_core::models::problem::*;

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

    let fleet_profiles = get_profile_map(api_problem);
    if fleet_profiles.len() > matrices.len() {
        return Err(format!(
            "not enough routing matrices specified for fleet profiles defined: \
             {} must be less or equal to {}",
            fleet_profiles.len(),
            matrices.len()
        ));
    }

    let matrix_data = matrices
        .iter()
        .enumerate()
        .map(|(idx, matrix)| {
            let profile = matrix.profile.as_ref().and_then(|p| fleet_profiles.get(p)).cloned().unwrap_or(idx as i32);
            (profile, matrix.timestamp.clone(), matrix)
        })
        .map(|(profile, timestamp, matrix)| {
            let (durations, distances) = if let Some(error_codes) = &matrix.error_codes {
                let mut durations: Vec<Duration> = Default::default();
                let mut distances: Vec<Distance> = Default::default();
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

    let matrix_profiles = matrix_data.iter().map(|data| data.profile).collect::<HashSet<_>>().len();
    if fleet_profiles.len() != matrix_profiles {
        return Err("amount of fleet profiles does not match matrix profiles".to_string());
    }

    create_matrix_transport_cost(matrix_data)
}

pub(crate) fn read_fleet(api_problem: &ApiProblem, props: &ProblemProperties, coord_index: &CoordIndex) -> Fleet {
    let profiles = get_profile_map(api_problem);
    let mut vehicles: Vec<Arc<Vehicle>> = Default::default();

    api_problem.fleet.vehicles.iter().for_each(|vehicle| {
        let costs = Costs {
            fixed: vehicle.costs.fixed.unwrap_or(0.),
            per_distance: vehicle.costs.distance,
            per_driving_time: vehicle.costs.time,
            per_waiting_time: vehicle.costs.time,
            per_service_time: vehicle.costs.time,
        };

        let profile = *profiles.get(&vehicle.profile).unwrap() as Profile;
        let mut areas = vehicle.limits.as_ref().and_then(|l| l.allowed_areas.as_ref()).map(|areas| {
            areas
                .iter()
                .map(|area| Area {
                    priority: area.priority,
                    outer_shape: area.outer_shape.iter().map(|l| l.to_lat_lng()).collect::<Vec<_>>(),
                })
                .collect::<Vec<_>>()
        });

        for (shift_index, shift) in vehicle.shifts.iter().enumerate() {
            let start = {
                let location = coord_index.get_by_loc(&shift.start.location).unwrap();
                let earliest = parse_time(&shift.start.earliest);
                let latest = shift.start.latest.as_ref().map(|time| parse_time(&time));
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

                if let Some(areas) = areas.take() {
                    dimens.set_value("areas", areas);
                }

                if props.has_multi_dimen_capacity {
                    dimens.set_capacity(MultiDimLoad::new(vehicle.capacity.clone()));
                } else {
                    dimens.set_capacity(SingleDimLoad::new(*vehicle.capacity.first().unwrap()));
                }
                add_vehicle_skills(&mut dimens, &vehicle.skills);

                vehicles.push(Arc::new(Vehicle { profile, costs: costs.clone(), dimens, details: details.clone() }));
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

pub fn read_limits(api_problem: &ApiProblem) -> Option<TravelLimitFunc> {
    let limits = api_problem.fleet.vehicles.iter().filter(|vehicle| vehicle.limits.is_some()).fold(
        HashMap::new(),
        |mut acc, vehicle| {
            let limits = vehicle.limits.as_ref().unwrap().clone();
            acc.insert(vehicle.type_id.clone(), (limits.max_distance, limits.shift_time));
            acc
        },
    );

    if limits.is_empty() {
        None
    } else {
        Some(Arc::new(move |actor: &Actor| {
            if let Some(limits) = limits.get(actor.vehicle.dimens.get_value::<String>("type_id").unwrap()) {
                (limits.0, limits.1)
            } else {
                (None, None)
            }
        }))
    }
}

fn get_profile_map(api_problem: &ApiProblem) -> HashMap<String, i32> {
    api_problem.fleet.profiles.iter().fold(Default::default(), |mut acc, profile| {
        if acc.get(&profile.name) == None {
            acc.insert(profile.name.clone(), acc.len() as i32);
        }
        acc
    })
}

fn add_vehicle_skills(dimens: &mut Dimensions, skills: &Option<Vec<String>>) {
    if let Some(skills) = skills {
        dimens.set_value("skills", HashSet::<String>::from_iter(skills.iter().cloned()));
    }
}
