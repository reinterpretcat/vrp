//! This module contains an implementation of Density-Based Spatial Clustering of Applications with
//! Noise (DBSCAN)

#[cfg(test)]
#[path = "../../../tests/unit/algorithms/clustering/dbscan_test.rs"]
mod dbscan_test;

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Represents a cluster of points.
pub type Cluster<'a, T> = Vec<&'a T>;

/// Creates clusters of points using DBSCAN (Density-Based Spatial Clustering of Applications with Noise).
/// `points`: A list of points to cluster.
/// `min_points`: The minimum number of points required to form a cluster.
/// `neighborhood_fn`: A function which returns neighbors of given point. It should return point itself.
pub fn create_clusters<'a, T, IS, FN, IR>(points: IS, min_points: usize, neighborhood_fn: FN) -> Vec<Cluster<'a, T>>
where
    T: Clone + Hash + Eq,
    IS: IntoIterator<Item = &'a T>,
    FN: Fn(&'a T) -> IR + 'a,
    IR: Iterator<Item = &'a T> + 'a,
{
    let mut point_types = HashMap::<&T, PointType>::new();
    let mut clusters = Vec::new();

    for point in points {
        if point_types.contains_key(point) {
            continue;
        }

        let mut neighbors = neighborhood_fn(point).collect::<Vec<_>>();
        let mut neighbors_index = neighbors.iter().cloned().collect::<HashSet<_>>();

        if neighbors.len() < min_points {
            point_types.insert(point, PointType::Noise);
        } else {
            let mut cluster = vec![point];
            point_types.insert(point, PointType::Clustered);

            let mut index = 0;
            while index < neighbors.len() {
                let point = neighbors[index];
                let point_type = point_types.get(point).cloned();

                if point_type.is_none() {
                    let other_neighbours = neighborhood_fn(point).collect::<Vec<_>>();
                    if other_neighbours.len() >= min_points {
                        neighbors
                            .extend(other_neighbours.iter().filter(|&point| !neighbors_index.contains(point)).cloned());
                        neighbors_index.extend(other_neighbours.into_iter());
                    }
                }

                match point_type {
                    Some(PointType::Clustered) => {}
                    _ => {
                        point_types.insert(point, PointType::Clustered);
                        cluster.push(point);
                    }
                }

                index += 1;
            }

            clusters.push(cluster);
        }
    }

    clusters
}

#[derive(Clone, Eq, PartialEq)]
enum PointType {
    Noise,
    Clustered,
}
