#[cfg(test)]
#[path = "../../../tests/unit/format/problem/fleet_reader_test.rs"]
mod fleet_reader_test;

use super::*;
use crate::Location as ApiLocation;
use crate::format::UnknownLocationFallback;
use crate::get_unique_locations;
use crate::utils::get_approx_transportation;
use std::collections::HashSet;
use vrp_core::construction::enablers::create_typed_actor_groups;
use vrp_core::construction::features::{VehicleCapacityDimension, VehicleSkillsDimension};
use vrp_core::models::common::*;
use vrp_core::models::problem::*;

pub(super) fn get_profile_index_map(api_problem: &ApiProblem) -> HashMap<String, usize> {
    api_problem.fleet.profiles.iter().fold(Default::default(), |mut acc, profile| {
        if !acc.contains_key(&profile.name) {
            acc.insert(profile.name.clone(), acc.len());
        }
        acc
    })
}

pub(super) fn create_transport_costs(
    api_problem: &ApiProblem,
    matrices: &[Matrix],
    coord_index: Arc<CoordIndex>,
) -> GenericResult<Arc<dyn TransportCost>> {
    if !matrices.iter().all(|m| m.profile.is_some()) && !matrices.iter().all(|m| m.profile.is_none()) {
        return Err("all matrices should have profile set or none of them".into());
    }

    if matrices.iter().any(|m| m.profile.is_none()) && matrices.iter().any(|m| m.timestamp.is_some()) {
        return Err("when timestamp is set, all matrices should have profile set".into());
    }

    let matrix_profiles = get_profile_index_map(api_problem);
    if matrix_profiles.len() > matrices.len() {
        return Err(format!(
            "not enough routing matrices specified for fleet profiles defined: \
             {} must be less or equal to {}",
            matrix_profiles.len(),
            matrices.len()
        )
        .into());
    }

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
                let err_fn = |i| move || GenericError::from(format!("invalid matrix index: {i}"));

                for (i, error) in error_codes.iter().enumerate() {
                    if *error > 0 {
                        durations.push(-1.);
                        distances.push(-1.);
                    } else {
                        durations.push(*matrix.travel_times.get(i).ok_or_else(err_fn(i))? as Float);
                        distances.push(*matrix.distances.get(i).ok_or_else(err_fn(i))? as Float);
                    }
                }

                (durations, distances)
            } else {
                (
                    matrix.travel_times.iter().map(|d| *d as Float).collect(),
                    matrix.distances.iter().map(|d| *d as Float).collect(),
                )
            };

            Ok(MatrixData::new(profile, timestamp.map(|t| parse_time(&t)), durations, distances))
        })
        .collect::<Result<Vec<_>, GenericError>>()?;

    let matrix_indices = matrix_data.iter().map(|data| data.index).collect::<HashSet<_>>().len();
    if matrix_profiles.len() != matrix_indices {
        return Err("amount of fleet profiles does not match matrix profiles".into());
    }

    if coord_index.has_custom() {
        create_matrix_transport_cost_with_fallback(matrix_data, UnknownLocationFallback::new(coord_index))
    } else {
        create_matrix_transport_cost(matrix_data)
    }
}

pub(super) fn read_fleet(api_problem: &ApiProblem, props: &ProblemProperties, coord_index: &CoordIndex) -> CoreFleet {
    let profile_indices = get_profile_index_map(api_problem);
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
        let min_tour_size = vehicle.limits.as_ref().and_then(|l| l.min_tour_size);

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

                dimens
                    .set_vehicle_type(vehicle.type_id.clone())
                    .set_shift_index(shift_index)
                    .set_vehicle_id(vehicle_id.to_string());

                if let Some(tour_size) = tour_size {
                    dimens.set_tour_size(tour_size);
                }

                if let Some(min_tour_size) = min_tour_size {
                    dimens.set_min_tour_size(min_tour_size);
                }

                if props.has_multi_dimen_capacity {
                    dimens.set_vehicle_capacity(MultiDimLoad::new(vehicle.capacity.clone()));
                } else {
                    dimens.set_vehicle_capacity(SingleDimLoad::new(*vehicle.capacity.first().unwrap()));
                }

                if let Some(skills) = vehicle.skills.as_ref() {
                    dimens.set_vehicle_skills(skills.iter().cloned().collect::<HashSet<_>>());
                }

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

    CoreFleet::new(drivers, vehicles, |actors| {
        create_typed_actor_groups(actors, |a| {
            a.vehicle.dimens.get_vehicle_type().cloned().expect("vehicle has no type defined")
        })
    })
}

/// Creates a matrices using approximation.
pub fn create_approx_matrices(problem: &ApiProblem) -> Vec<Matrix> {
    const DEFAULT_SPEED: Float = 10.;
    // get each speed value once
    let speeds = problem
        .fleet
        .profiles
        .iter()
        .map(|profile| profile.speed.unwrap_or(DEFAULT_SPEED))
        .map(|speed| speed.to_bits())
        .collect::<HashSet<_>>();
    let speeds = speeds.into_iter().map(Float::from_bits).collect::<Vec<_>>();

    let locations = get_unique_locations(problem)
        .into_iter()
        .filter(|location| !matches!(location, ApiLocation::Custom { .. }))
        .collect::<Vec<_>>();
    let approx_data = get_approx_transportation(&locations, speeds.as_slice());

    problem
        .fleet
        .profiles
        .iter()
        .map(move |profile| {
            let speed = profile.speed.unwrap_or(DEFAULT_SPEED);
            let idx = speeds.iter().position(|&s| s == speed).expect("Cannot find profile speed");

            Matrix {
                profile: Some(profile.name.clone()),
                timestamp: None,
                travel_times: approx_data[idx].0.clone(),
                distances: approx_data[idx].1.clone(),
                error_codes: None,
            }
        })
        .collect()
}
