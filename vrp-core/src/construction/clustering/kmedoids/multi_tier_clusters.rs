#[cfg(test)]
#[path = "../../../../tests/unit/construction/clustering/kmedoids/multi_tier_clusters_test.rs"]
mod multi_tier_clusters_test;

use crate::algorithms::clustering::kmedoids::create_k_medoids;
use crate::models::common::Profile;
use crate::prelude::{GenericResult, Location, TransportCost};
use rosomaxa::prelude::Random;
use std::collections::HashMap;

/// Represents a single tier of multiple location clusters.
pub type LocationClusters = HashMap<Location, Vec<Location>>;

/// Creates multi-tier clusters of jobs using DBSCAN algorithm.
/// Returns tiers of clusters starting from the finer grained (lowest epsilon).
pub fn create_multi_tier_clusters(
    profile: Profile,
    transport: &(dyn TransportCost),
    random: &dyn Random,
) -> GenericResult<Vec<LocationClusters>> {
    let size = transport.size();
    let limit = size / 3;

    let points = (0..size).collect::<Vec<_>>();

    let location_clusters = [2, 3, 4, 5, 8, 10, 12, 16, 32, 64]
        .iter()
        .filter(|&&k| k <= limit)
        .map(|&k| create_k_medoids(&points, k, random, |from, to| transport.distance_approx(&profile, *from, *to)))
        .filter(|clusters| !clusters.is_empty())
        .collect();

    Ok(location_clusters)
}
