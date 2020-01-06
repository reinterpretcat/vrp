//! A helper module for processing geo coordinates in problem and solution.

use crate::json::problem::{JobVariant, Problem};
use crate::json::Location;
use std::cmp::Ordering::Less;
use std::collections::HashMap;
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
        problem.plan.jobs.iter().for_each(|job| match &job {
            JobVariant::Single(job) => {
                if let Some(pickup) = &job.places.pickup {
                    index.add(&pickup.location);
                }
                if let Some(delivery) = &job.places.delivery {
                    index.add(&delivery.location);
                }
            }
            JobVariant::Multi(job) => {
                job.places.pickups.iter().for_each(|pickup| {
                    index.add(&pickup.location);
                });
                job.places.deliveries.iter().for_each(|delivery| {
                    index.add(&delivery.location);
                });
            }
        });

        // process fleet
        problem.fleet.types.iter().for_each(|vehicle| {
            vehicle.shifts.iter().for_each(|shift| {
                index.add(&shift.start.location);

                if let Some(end) = &shift.end {
                    index.add(&end.location);
                }

                if let Some(breaks) = &shift.breaks {
                    breaks.iter().for_each(|vehicle_break| {
                        if let Some(location) = &vehicle_break.location {
                            index.add(location);
                        }
                    });
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
            let value = self.direct_index.len();
            self.direct_index.insert(location.clone(), value);
            self.reverse_index.insert(value, location.clone());
        }
    }

    pub fn get_by_loc(&self, location: &Location) -> Option<usize> {
        self.direct_index.get(location).cloned()
    }

    pub fn get_by_idx(&self, index: &usize) -> Option<Location> {
        self.reverse_index.get(index).cloned()
    }

    pub fn unique(&self) -> Vec<Location> {
        let mut sorted_pairs: Vec<_> = self.reverse_index.iter().collect();
        sorted_pairs.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(Less));
        sorted_pairs.iter().map(|pair| pair.1.clone()).collect()
    }
}

impl Eq for Location {}

impl PartialEq for Location {
    fn eq(&self, other: &Self) -> bool {
        self.lat == other.lat && self.lng == other.lng
    }
}

impl Hash for Location {
    fn hash<H: Hasher>(&self, state: &mut H) {
        write_hash(self.lat, state);
        write_hash(self.lng, state);
    }
}

fn write_hash<H: Hasher>(value: f64, state: &mut H) {
    let value: u64 = unsafe { std::mem::transmute(value) };
    state.write_u64(value);
}
