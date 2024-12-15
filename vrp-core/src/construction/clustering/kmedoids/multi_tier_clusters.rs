// TODO remove allow macros

#![allow(dead_code)]
#![allow(unused_variables)]

#[cfg(test)]
#[path = "../../../../tests/unit/construction/clustering/kmedoids/multi_tier_clusters_test.rs"]
mod multi_tier_clusters_test;

use crate::algorithms::clustering::dbscan::create_clusters;
use crate::models::common::{Distance, Profile};
use crate::prelude::{GenericResult, Location, TransportCost};
use rosomaxa::algorithms::math::Remedian;
use rosomaxa::utils::parallel_collect;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// Represents a single tier of multiple location clusters.
pub type LocationClusters = Vec<HashSet<Location>>;

/// Creates multi-tier clusters of jobs using DBSCAN algorithm.
/// Returns tiers of clusters starting from the finer grained (lowest epsilon).
pub fn create_multi_tier_clusters(
    profile: Profile,
    transport: &(dyn TransportCost),
    min_points: Option<usize>,
) -> GenericResult<Vec<LocationClusters>> {
    let min_points = min_points.unwrap_or(3).max(2);
    let locations = (0..transport.size()).collect::<Vec<_>>();

    let mut epsilon_distances = EpsilonDistances::default();
    let distances = parallel_collect(locations.as_slice(), |outer| {
        let mut remedians = DistanceRemedians::default();

        let mut distances = locations
            .iter()
            .filter(|&inner| inner != outer)
            .map(|inner| (*inner, transport.distance_approx(&profile, *outer, *inner)))
            .inspect(|(_, distance)| remedians.add_main_observation(*distance))
            .collect::<Vec<_>>();

        distances.sort_unstable_by(|(_, a), (_, b)| a.total_cmp(b));

        remedians.add_extra_observations(distances.as_slice());

        (outer, remedians, distances)
    })
    // TODO does rayon preserve order?
    .into_iter()
    .inspect(|(_, other_remedians, _)| epsilon_distances.add(other_remedians))
    .map(|(outer, _, distances)| (outer, distances))
    .collect::<HashMap<_, _>>();

    Ok(epsilon_distances
        .into_iter()
        .map(|epsilon| {
            let neighborhood_fn = |location| {
                distances[location].iter().filter(|(_, distance)| *distance < epsilon).map(|(other, _)| other)
            };
            create_clusters(locations.iter(), min_points, neighborhood_fn)
                .into_iter()
                .map(|cluster| cluster.into_iter().copied().collect::<HashSet<_>>())
                .collect()
        })
        .collect())
}

type DistanceRemedian = Remedian<Distance, fn(&Distance, &Distance) -> Ordering>;
const MAX_NUM_OF_TIERS: usize = 4;
const FIRST_TIER_TOP: usize = 4;

struct DistanceRemedians {
    remedians: Vec<DistanceRemedian>,
}

impl DistanceRemedians {
    fn default() -> Self {
        let remedians = (0..MAX_NUM_OF_TIERS)
            .map(|_| DistanceRemedian::new(11, 7, |a: &Distance, b: &Distance| a.total_cmp(b)))
            .collect::<Vec<_>>();

        Self { remedians }
    }

    fn add_main_observation(&mut self, distance: Distance) {
        self.remedians[0].add_observation(distance);
    }

    fn add_extra_observations(&mut self, distances: &[(usize, Distance)]) {
        (1..MAX_NUM_OF_TIERS)
            .filter_map(|idx| {
                // 4  8  16
                let top = FIRST_TIER_TOP * 2_usize.pow((idx - 1) as u32);
                distances.get(top).map(|(_, value)| (idx, value))
            })
            .for_each(|(idx, value)| {
                self.remedians[idx].add_observation(*value);
            });
    }
}

struct EpsilonDistances {
    distances: Vec<Vec<Distance>>,
}

impl EpsilonDistances {
    fn default() -> Self {
        Self { distances: vec![Vec::default(); MAX_NUM_OF_TIERS] }
    }

    fn add(&mut self, remedian: &DistanceRemedians) {
        self.distances
            .iter_mut()
            .zip(remedian.remedians.iter())
            .filter_map(|(distances, remedian)| remedian.approx_median().map(|v| (distances, v)))
            .for_each(|(distances, distance)| {
                distances.push(distance);
            });
    }

    fn into_iter(self) -> impl Iterator<Item = Distance> {
        let mut avg_distances = self
            .distances
            .into_iter()
            .map(|distances| {
                if distances.is_empty() {
                    Distance::default()
                } else {
                    let sum = distances.iter().sum::<Distance>();
                    sum / distances.len() as Distance
                }
            })
            .collect::<Vec<_>>();

        avg_distances.sort_by(|a, b| a.total_cmp(b));
        avg_distances.into_iter()
    }
}
