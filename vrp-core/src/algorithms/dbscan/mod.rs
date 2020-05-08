#[cfg(test)]
#[path = "../../../tests/unit/algorithms/dbscan/dbscan_test.rs"]
mod dbscan_test;

use hashbrown::{HashMap, HashSet};
use std::hash::Hash;

/// Represents a cluster of items.
type Cluster<'a, T> = Vec<&'a T>;

/// A function which returns neighbors of given item with given epsilon.
type NeighborhoodFn<'a, T> = Box<dyn Fn(&'a T, f64) -> Box<dyn Iterator<Item = &'a T> + 'a> + 'a>;

/// Creates clusters of items using DBSCAN (Density-Based Spatial Clustering of Applications with Noise) algorithm.
fn create_clusters<'a, T>(
    items: &[&'a T],
    eps: f64,
    min_items: usize,
    neighborhood_fn: &NeighborhoodFn<'a, T>,
) -> Vec<Cluster<'a, T>>
where
    T: Hash + Eq,
{
    let mut item_types = HashMap::<&T, ItemType>::new();
    let mut clusters = Vec::new();

    for item in items {
        if item_types.get(item).is_some() {
            continue;
        }

        let mut neighbors = neighborhood_fn(item, eps).collect::<Vec<_>>();
        if neighbors.len() < min_items {
            item_types.insert(item, ItemType::Noise);
        } else {
            let mut cluster = Vec::new();

            cluster.push(*item);
            item_types.insert(item, ItemType::Clustered);

            let mut index = 0;
            while index < neighbors.len() {
                let item = neighbors[index];

                let item_type = item_types.get(item);

                if item_type.is_none() {
                    let other_neighbours = neighborhood_fn(item, eps).collect::<Vec<_>>();
                    if other_neighbours.len() >= min_items {
                        let set = neighbors.iter().cloned().collect::<HashSet<_>>();
                        neighbors.extend(other_neighbours.into_iter().filter(move |item| !set.contains(item)));
                    }
                }

                match item_type {
                    Some(item_type) if *item_type == ItemType::Clustered => {}
                    _ => {
                        item_types.insert(item, ItemType::Clustered);
                        cluster.push(item);
                    }
                }

                index += 1;
            }

            clusters.push(cluster);
        }
    }

    clusters
}

#[derive(Eq, PartialEq)]
enum ItemType {
    Noise,
    Clustered,
}
