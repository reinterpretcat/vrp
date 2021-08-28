//! This module contains an implementation of Density-Based Spatial Clustering of Applications with
//! Noise (DBSCAN)

#[cfg(test)]
#[path = "../../../tests/unit/algorithms/dbscan/dbscan_test.rs"]
mod dbscan_test;

use hashbrown::{HashMap, HashSet};
use std::hash::Hash;

/// Represents a cluster of points.
pub type Cluster<'a, T> = Vec<&'a T>;

/// A function which returns neighbors of given point with given epsilon.
pub type NeighborhoodFn<'a, T> = Box<dyn Fn(&'a T, f64) -> Box<dyn Iterator<Item = &'a T> + 'a> + 'a>;

/// Creates clusters of points using DBSCAN (Density-Based Spatial Clustering of Applications with Noise)
/// algorithm. NOTE: `neighborhood_fn` shall return point itself.
pub fn create_clusters<'a, T>(
    points: &'a [T],
    epsilon: f64,
    min_points: usize,
    neighborhood_fn: &NeighborhoodFn<'a, T>,
) -> Vec<Cluster<'a, T>>
where
    T: Hash + Eq,
{
    let mut point_types = HashMap::<&T, PointType>::new();
    let mut clusters = Vec::new();

    for point in points {
        if point_types.get(point).is_some() {
            continue;
        }

        let mut neighbors = neighborhood_fn(point, epsilon).collect::<Vec<_>>();
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
                    let other_neighbours = neighborhood_fn(point, epsilon).collect::<Vec<_>>();
                    if other_neighbours.len() >= min_points {
                        let set = neighbors.iter().cloned().collect::<HashSet<_>>();
                        neighbors.extend(other_neighbours.into_iter().filter(move |point| !set.contains(point)));
                    }
                }

                match point_type {
                    Some(point_type) if point_type == PointType::Clustered => {}
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
