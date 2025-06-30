//! This module contains a basic K-Medoids algorithm implementation.

#[cfg(test)]
#[path = "../../../tests/unit/algorithms/clustering/kmedoids_test.rs"]
mod kmedoids_test;

use rosomaxa::utils::{fold_reduce, map_reduce};
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

/// A data point type that can be used with the K-Medoids algorithm.
pub trait Point: Hash + Eq + PartialEq + Clone + Send + Sync {}

/// Creates clusters of data points using the K-Medoids algorithm.
pub fn create_kmedoids<P, F>(points: &[P], k: usize, distance_fn: F) -> HashMap<P, Vec<P>>
where
    P: Point,
    F: Fn(&P, &P) -> f64 + Send + Sync,
{
    const MAX_ITERATIONS: usize = 200;

    if points.is_empty() {
        return HashMap::default();
    }

    KMedoids::new(k, MAX_ITERATIONS, distance_fn).calculate(points)
}

/// Creates hierarchical clusters of data points using the K-Medoids algorithm.
pub fn create_hierarchical_kmedoids<P, F>(points: &[P], max_tiers: usize, distance_fn: F) -> Vec<HashMap<P, Vec<P>>>
where
    P: Point,
    F: Fn(&P, &P) -> f64 + Send + Sync + Clone,
{
    if points.is_empty() {
        return Vec::default();
    }

    const K_PER_TIER: usize = 2;

    (0..max_tiers)
        .scan(vec![(Option::<P>::None, points.to_vec())], |current_clusters, _| {
            let mut current_tier_clusters = HashMap::new();
            let mut next_tier_clusters = Vec::new();

            for (medoid, cluster_data) in std::mem::take(current_clusters).into_iter() {
                // not enough data for clustering, simply propagate it to the next tier
                if cluster_data.len() < K_PER_TIER {
                    current_tier_clusters.insert(medoid.clone().expect("should be set"), cluster_data.clone());
                    next_tier_clusters.push((medoid, cluster_data));
                    continue;
                } else {
                    let new_clusters = create_kmedoids(&cluster_data, K_PER_TIER, distance_fn.clone());
                    for (medoid, cluster) in new_clusters.iter() {
                        next_tier_clusters.push((Some(medoid.clone()), cluster.clone()));
                    }
                    current_tier_clusters.extend(new_clusters);
                }
            }

            *current_clusters = next_tier_clusters;

            if current_tier_clusters.is_empty() { None } else { Some(current_tier_clusters) }
        })
        // do not go on the level where all clusters will be of size 1
        .take_while(|clusters| clusters.iter().any(|(_, cluster)| cluster.len() > 2))
        .collect()
}

struct KMedoids<P, F> {
    distance_fn: F,
    k: usize,
    max_iterations: usize,
    phantom_data: PhantomData<P>,
}

impl<P, F> KMedoids<P, F>
where
    P: Point,
    F: Fn(&P, &P) -> f64 + Send + Sync,
{
    /// Creates a new K-Medoids instance.
    pub fn new(k: usize, max_iterations: usize, distance_fn: F) -> Self {
        Self { k, max_iterations, distance_fn, phantom_data: Default::default() }
    }

    fn initialize_medoids(&self, data: &[P]) -> Option<Vec<P>> {
        let mut medoids = Vec::with_capacity(self.k);

        // 1. select first medoid as the "most central" point in the dataset.
        let first_medoid = data.iter().min_by(|a, b| {
            let (sum_a, sum_b) = map_reduce(
                data,
                |p| ((self.distance_fn)(a, p), (self.distance_fn)(b, p)),
                || (0., 0.),
                |(sum_a, sum_b), (a, b)| (sum_a + a, sum_b + b),
            );

            let (avg_a, avg_b) = (sum_a / data.len() as f64, sum_b / data.len() as f64);

            avg_a.total_cmp(&avg_b)
        })?;

        medoids.push(first_medoid.clone());

        // 2. select remaining medoids where each subsequent medoid is selected to maximize the
        // minimum distance to any already-selected medoid. This ensures the medoids are spread out
        // while being deterministic.
        while medoids.len() < self.k {
            let next_medoid = fold_reduce(
                data,
                || (f64::NEG_INFINITY, Option::<P>::None),
                |(max_distance, best_medoid), point| {
                    if medoids.contains(point) {
                        return (max_distance, best_medoid);
                    }

                    medoids
                        .iter()
                        .map(|m| (self.distance_fn)(point, m))
                        .min_by(|a, b| a.total_cmp(b))
                        .map(|distance| (distance, Some(point.clone())))
                        .unwrap_or((max_distance, best_medoid))
                },
                |left, right| {
                    if left.0 > right.0 { left } else { right }
                },
            )
            .1?;

            medoids.push(next_medoid.clone());
        }

        Some(medoids)
    }

    fn assign_points_to_medoids(&self, data: &[P], medoids: &[P]) -> HashMap<P, Vec<P>> {
        fold_reduce(
            data,
            || HashMap::<P, Vec<P>>::new(),
            |mut clusters, point| {
                let nearest_medoid = medoids
                    .iter()
                    .min_by(|m1, m2| (self.distance_fn)(point, m1).total_cmp(&(self.distance_fn)(point, m2)))
                    .expect("cannot find nearest medoid");

                clusters.entry(nearest_medoid.clone()).or_default().push(point.clone());

                clusters
            },
            |mut acc, clusters| {
                for (key, value) in clusters {
                    acc.entry(key).or_default().extend(value);
                }
                acc
            },
        )
    }

    fn update_medoids(&self, clusters: &HashMap<P, Vec<P>>) -> Vec<P> {
        clusters
            .values()
            .map(|points| {
                points
                    .iter()
                    .min_by(|&p1, &p2| {
                        let cost1: f64 = points.iter().map(|point| (self.distance_fn)(point, p1)).sum();
                        let cost2: f64 = points.iter().map(|point| (self.distance_fn)(point, p2)).sum();

                        cost1.total_cmp(&cost2)
                    })
                    .expect("cannot find medoid")
                    .clone()
            })
            .collect()
    }

    pub fn calculate(&self, data: &[P]) -> HashMap<P, Vec<P>> {
        let Some(mut medoids) = self.initialize_medoids(data) else {
            return HashMap::default();
        };

        for _ in 0..self.max_iterations {
            let clusters = self.assign_points_to_medoids(data, &medoids);
            let new_medoids = self.update_medoids(&clusters);

            if new_medoids == medoids {
                return clusters;
            }

            medoids = new_medoids;
        }

        self.assign_points_to_medoids(data, &medoids)
    }
}

impl Point for usize {}
