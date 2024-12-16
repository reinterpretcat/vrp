//! This module contains a basic K-Medoids algorithm implementation.

#[cfg(test)]
#[path = "../../../tests/unit/algorithms/clustering/kmedoids_test.rs"]
mod kmedoids_test;

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

/// A data point type that can be used with the K-Medoids algorithm.
pub trait Point: Hash + Eq + PartialEq + Clone {}

/// Creates clusters of data points using the K-Medoids algorithm.
pub fn create_kmedoids<P, F>(points: &[P], k: usize, distance_fn: F) -> HashMap<P, Vec<P>>
where
    P: Point,
    F: Fn(&P, &P) -> f64,
{
    const MAX_ITERATIONS: usize = 200;

    if points.is_empty() {
        return HashMap::default();
    }

    KMedoids::new(k, MAX_ITERATIONS, distance_fn).calculate(points)
}

/// Creates hierarchical clusters of data points using the K-Medoids algorithm.
pub fn create_hierarchical_kmedoids<P, F>(points: &[P], tiers: usize, distance_fn: F) -> Vec<HashMap<P, Vec<P>>>
where
    P: Point,
    F: Fn(&P, &P) -> f64 + Clone,
{
    if points.is_empty() {
        return Vec::default();
    }

    const K_PER_TIER: usize = 2;

    (0..tiers)
        .scan(vec![(None, points.to_vec())], |current_clusters, _| {
            let mut current_tier_clusters = HashMap::new();
            let mut next_tier_clusters = Vec::new();

            for (medoid, cluster_data) in std::mem::take(current_clusters).into_iter() {
                // not enough data for clustering, simply propagate it to the next tier
                if cluster_data.len() < K_PER_TIER {
                    current_tier_clusters.insert(medoid.expect("should be set"), cluster_data);
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

            Some(current_tier_clusters)
        })
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
    F: Fn(&P, &P) -> f64,
{
    /// Creates a new K-Medoids instance.
    pub fn new(k: usize, max_iterations: usize, distance_fn: F) -> Self {
        Self { k, max_iterations, distance_fn, phantom_data: Default::default() }
    }

    fn initialize_medoids(&self, data: &[P]) -> Option<Vec<P>> {
        let mut medoids = Vec::with_capacity(self.k);

        // select first medoid
        let first_medoid = data.iter().min_by(|a, b| {
            let (sum_a, sum_b) = data
                .iter()
                .map(|p| ((self.distance_fn)(a, p), (self.distance_fn)(b, p)))
                .fold((0., 0.), |(sum_a, sum_b), (a, b)| (sum_a + a, sum_b + b));

            let (avg_a, avg_b) = (sum_a / data.len() as f64, sum_b / data.len() as f64);

            avg_a.total_cmp(&avg_b)
        })?;
        medoids.push(first_medoid.clone());

        // select the remaining medoids
        while medoids.len() < self.k {
            let next_medoid = data.iter().filter(|p| !medoids.contains(p)).max_by(|a, b| {
                let min_distance_a = medoids.iter().map(|m| (self.distance_fn)(a, m)).fold(f64::INFINITY, f64::min);
                let min_distance_b = medoids.iter().map(|m| (self.distance_fn)(b, m)).fold(f64::INFINITY, f64::min);

                min_distance_a.total_cmp(&min_distance_b)
            })?;
            medoids.push(next_medoid.clone());
        }

        Some(medoids)
    }

    fn assign_points_to_medoids(&self, data: &[P], medoids: &[P]) -> HashMap<P, Vec<P>> {
        data.iter().fold(HashMap::new(), |mut clusters, point| {
            let nearest_medoid = medoids
                .iter()
                .min_by(|m1, m2| (self.distance_fn)(point, m1).total_cmp(&(self.distance_fn)(point, m2)))
                .unwrap();

            clusters.entry(nearest_medoid.clone()).or_default().push(point.clone());

            clusters
        })
    }

    fn update_medoids(&self, clusters: &HashMap<P, Vec<P>>) -> Vec<P> {
        clusters
            .iter()
            .map(|(_, points)| {
                points
                    .iter()
                    .min_by(|&p1, &p2| {
                        let cost1: f64 = points.iter().map(|point| (self.distance_fn)(point, p1)).sum();
                        let cost2: f64 = points.iter().map(|point| (self.distance_fn)(point, p2)).sum();

                        cost1.total_cmp(&cost2)
                    })
                    .unwrap()
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
