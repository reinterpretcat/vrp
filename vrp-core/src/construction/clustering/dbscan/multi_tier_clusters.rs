use super::*;
use crate::models::common::Distance;
use crate::prelude::{Location, TransportCost};
use rosomaxa::algorithms::math::Remedian;
use rosomaxa::utils::parallel_collect;
use std::cmp::Ordering;
use std::collections::HashMap;

/// Represents a single tier of multiple location clusters.
pub type LocationClusters = Vec<HashSet<Location>>;

/// Creates multi-tier clusters of jobs using DBSCAN algorithm.
/// Returns tiers of clusters starting from the finer grained (lowest epsilon).
pub fn create_multi_tier_clusters<'a, FN, IR>(
    profile: Profile,
    transport: &(dyn TransportCost),
    min_points: Option<usize>,
) -> GenericResult<Vec<LocationClusters>>
where
    FN: Fn(&Profile, &Job) -> IR + 'a,
    IR: Iterator<Item = (&'a Job, Cost)> + 'a,
{
    let min_points = min_points.unwrap_or(3).max(2);
    let locations = (0..transport.size()).collect::<Vec<_>>();

    let mut remedians = DistanceRemedians::default();

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
    .inspect(|(_, other_remedians, _)| remedians.merge(other_remedians))
    .map(|(outer, _, distances)| (outer, distances))
    .collect::<HashMap<_, _>>();

    Ok(remedians
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

struct DistanceRemedians {
    remedians: Vec<DistanceRemedian>,
}

impl DistanceRemedians {
    const MAX_NUM_OF_TIERS: usize = 4;
    const FIRST_TIER_TOP: usize = 4;

    fn default() -> Self {
        let remedians = (0..Self::MAX_NUM_OF_TIERS)
            .map(|_| DistanceRemedian::new(11, 7, |a: &Distance, b: &Distance| a.total_cmp(b)))
            .collect::<Vec<_>>();

        Self { remedians }
    }

    fn add_main_observation(&mut self, distance: Distance) {
        self.remedians[0].add_observation(distance);
    }

    fn add_extra_observations(&mut self, distances: &[(usize, Distance)]) {
        (1..Self::MAX_NUM_OF_TIERS)
            .filter_map(|idx| {
                // 4  8  16
                let top = Self::FIRST_TIER_TOP * 2_usize.pow((idx - 1) as u32);
                distances.get(top).map(|(_, value)| (idx, value))
            })
            .for_each(|(idx, value)| {
                self.remedians[idx].add_observation(*value);
            });
    }

    fn merge(&mut self, other: &Self) {
        self.remedians
            .iter_mut()
            .zip(other.remedians.iter())
            .filter_map(|(a, b)| b.approx_median().map(|v| (a, v)))
            .for_each(|(a, v)| {
                a.add_observation(v);
            });
    }

    fn into_iter(self) -> impl Iterator<Item = Distance> {
        let mut distances = self.remedians.into_iter().filter_map(|r| r.approx_median()).collect::<Vec<_>>();
        distances.sort_by(|a, b| a.total_cmp(b));

        distances.into_iter()
    }
}
