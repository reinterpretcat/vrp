use crate::extensions::MultiDimensionalCapacity;
use crate::json::coord_index::CoordIndex;
use crate::json::problem::reader::{add_skills, parse_time, ApiProblem, ProblemProperties};
use crate::json::problem::Matrix;
use core::construction::constraints::CapacityDimension;
use core::construction::constraints::TravelLimitFunc;
use core::models::common::{Dimensions, Distance, Duration, IdDimension, Profile, TimeWindow, ValueDimension};
use core::models::problem::{Actor, Costs, Driver, Fleet, MatrixTransportCost, Vehicle, VehicleDetail};
use std::collections::HashMap;
use std::sync::Arc;

pub fn create_transport_costs(matrices: &Vec<Matrix>) -> MatrixTransportCost {
    let mut all_durations: Vec<Vec<Duration>> = Default::default();
    let mut all_distances: Vec<Vec<Distance>> = Default::default();

    matrices.iter().for_each(|matrix| {
        if let Some(error_codes) = &matrix.error_codes {
            let mut profile_durations: Vec<Duration> = Default::default();
            let mut profile_distances: Vec<Distance> = Default::default();
            for (i, error) in error_codes.iter().enumerate() {
                if *error > 0 {
                    profile_durations.push(-1.);
                    profile_distances.push(-1.);
                } else {
                    profile_durations.push(*matrix.travel_times.get(i).unwrap() as f64);
                    profile_distances.push(*matrix.distances.get(i).unwrap() as f64);
                }
            }
            all_durations.push(profile_durations);
            all_distances.push(profile_distances);
        } else {
            all_durations.push(matrix.travel_times.iter().map(|d| *d as f64).collect());
            all_distances.push(matrix.distances.iter().map(|d| *d as f64).collect());
        }
    });

    MatrixTransportCost::new(all_durations, all_distances)
}

pub fn read_fleet(api_problem: &ApiProblem, props: &ProblemProperties, coord_index: &CoordIndex) -> Fleet {
    let profiles = get_profile_map(api_problem);
    let mut vehicles: Vec<Vehicle> = Default::default();

    api_problem.fleet.types.iter().for_each(|vehicle| {
        let costs = Costs {
            fixed: vehicle.costs.fixed.unwrap_or(0.),
            per_distance: vehicle.costs.distance,
            per_driving_time: vehicle.costs.time,
            per_waiting_time: vehicle.costs.time,
            per_service_time: vehicle.costs.time,
        };

        let profile = *profiles.get(&vehicle.profile).unwrap() as Profile;

        for (shift_index, shift) in vehicle.shifts.iter().enumerate() {
            let start = {
                let location = coord_index.get_by_loc(&shift.start.location).unwrap();
                let time = parse_time(&shift.start.time);
                (location, time)
            };

            let end = shift.end.as_ref().map_or(None, |end| {
                let location = coord_index.get_by_loc(&end.location).unwrap();
                let time = parse_time(&end.time);
                Some((location, time))
            });

            let details = vec![VehicleDetail {
                start: Some(start.0),
                end: end.map_or(None, |end| Some(end.0)),
                time: Some(TimeWindow::new(start.1, end.map_or(std::f64::MAX, |end| end.1))),
            }];

            (1..vehicle.amount + 1).for_each(|number| {
                let mut dimens: Dimensions = Default::default();
                dimens.insert("type_id".to_owned(), Box::new(vehicle.id.clone()));
                dimens.insert("shift_index".to_owned(), Box::new(shift_index));
                dimens.set_id(format!("{}_{}", vehicle.id, number.to_string()).as_str());

                if props.has_multi_dimen_capacity {
                    dimens.set_capacity(MultiDimensionalCapacity::new(vehicle.capacity.clone()));
                } else {
                    dimens.set_capacity(*vehicle.capacity.first().unwrap());
                }
                add_skills(&mut dimens, &vehicle.skills);

                vehicles.push(Vehicle { profile, costs: costs.clone(), dimens, details: details.clone() });
            });
        }
    });

    let fake_driver = Driver {
        costs: Costs {
            fixed: 0.0,
            per_distance: 0.0,
            per_driving_time: 0.0,
            per_waiting_time: 0.0,
            per_service_time: 0.0,
        },
        dimens: Default::default(),
        details: vec![],
    };

    Fleet::new(vec![fake_driver], vehicles)
}

pub fn read_limits(api_problem: &ApiProblem) -> Option<TravelLimitFunc> {
    let limits = api_problem.fleet.types.iter().filter(|vehicle| vehicle.limits.is_some()).fold(
        HashMap::new(),
        |mut acc, vehicle| {
            let limits = vehicle.limits.as_ref().unwrap().clone();
            acc.insert(vehicle.id.clone(), (limits.max_distance, limits.shift_time));
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

fn get_profile_map(api_problem: &ApiProblem) -> HashMap<String, usize> {
    api_problem.fleet.profiles.iter().fold(Default::default(), |mut acc, profile| {
        if acc.get(&profile.name) == None {
            acc.insert(profile.name.clone(), acc.len());
        }
        acc
    })
}
