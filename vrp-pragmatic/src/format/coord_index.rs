//! A helper module for processing geo coordinates in problem and solution.

use crate::format::problem::Problem;
use crate::format::Location;
use hashbrown::HashMap;
use std::cmp::Ordering::Less;
use std::hash::{Hash, Hasher};

/// A helper struct which keeps track of coordinate mapping.
pub struct CoordIndex {
    direct_index: HashMap<Location, usize>,
    reverse_index: HashMap<usize, Location>,
}

impl CoordIndex {
    pub fn new(problem: &Problem) -> Self {
        let mut index = Self { direct_index: Default::default(), reverse_index: Default::default() };

        // process plan
        problem.plan.jobs.iter().for_each(|job| {
            job.pickups
                .iter()
                .chain(job.deliveries.iter())
                .chain(job.replacements.iter())
                .chain(job.services.iter())
                .flat_map(|tasks| tasks.iter().flat_map(|task| task.places.iter()))
                .for_each(|place| {
                    index.add(&place.location);
                });
        });

        // process fleet
        problem.fleet.vehicles.iter().for_each(|vehicle| {
            vehicle.shifts.iter().for_each(|shift| {
                index.add(&shift.start.location);

                if let Some(end) = &shift.end {
                    index.add(&end.location);
                }

                if let Some(dispatch) = &shift.dispatch {
                    dispatch.iter().for_each(|dispatch| index.add(&dispatch.location));
                }

                if let Some(breaks) = &shift.breaks {
                    breaks
                        .iter()
                        .flat_map(|vehicle_break| vehicle_break.places.iter())
                        .filter_map(|place| place.location.as_ref())
                        .for_each(|location| index.add(location));
                }

                if let Some(reloads) = &shift.reloads {
                    reloads.iter().for_each(|reload| index.add(&reload.location));
                }
            });
        });

        index
    }

    pub fn add(&mut self, location: &Location) {
        if self.direct_index.get(location).is_none() {
            let value = match location {
                Location::Coordinate { lat: _, lng: _ } => self.direct_index.len(),
                Location::Reference { index } => *index,
            };

            self.direct_index.insert(location.clone(), value);
            self.reverse_index.insert(value, location.clone());
        }
    }

    pub fn get_by_loc(&self, location: &Location) -> Option<usize> {
        self.direct_index.get(location).cloned()
    }

    pub fn get_by_idx(&self, index: usize) -> Option<Location> {
        self.reverse_index.get(&index).cloned()
    }

    pub fn unique(&self) -> Vec<Location> {
        let mut sorted_pairs: Vec<_> = self.reverse_index.iter().collect();
        sorted_pairs.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(Less));
        sorted_pairs.iter().map(|pair| pair.1.clone()).collect()
    }

    pub fn max_index(&self) -> Option<usize> {
        self.reverse_index.keys().max().cloned()
    }

    /// Returns types of locations in form (has_coordinates, has_indices).
    pub fn get_used_types(&self) -> (bool, bool) {
        self.direct_index.iter().fold((false, false), |(has_coordinates, has_indices), (location, _)| match location {
            Location::Coordinate { lat: _, lng: _ } => (true, has_indices),
            Location::Reference { index: _ } => (has_coordinates, true),
        })
    }
}

impl Eq for Location {}

impl PartialEq for Location {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Location::Coordinate { lat: l_lat, lng: l_lng }, Location::Coordinate { lat: r_lat, lng: r_lng }) => {
                (l_lat - r_lat).abs() < std::f64::EPSILON && (l_lng - r_lng).abs() < std::f64::EPSILON
            }
            (Location::Reference { index: left }, Location::Reference { index: right }) => left == right,
            _ => false,
        }
    }
}

impl Hash for Location {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Location::Coordinate { lat, lng } => {
                state.write_u64(lat.to_bits());
                state.write_u64(lng.to_bits());
            }
            Location::Reference { index } => {
                state.write_usize(*index);
            }
        }
    }
}
