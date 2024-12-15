//! This module contains a basic K-Medoids algorithm implementation.

#[cfg(test)]
#[path = "../../../tests/unit/algorithms/clustering/kmedoids_test.rs"]
mod kmedoids_test;

use rosomaxa::prelude::Random;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

/// A data point type that can be used with the K-Medoids algorithm.
pub trait Point: Hash + Eq + PartialEq + Clone {}

/// Creates clusters of data points using the K-Medoids algorithm.
pub fn create_k_medoids<P, F>(points: &[P], k: usize, random: &dyn Random, distance_fn: F) -> HashMap<P, Vec<P>>
where
    P: Point,
    F: Fn(&P, &P) -> f64,
{
    const MAX_ITERATIONS: usize = 200;

    if points.is_empty() {
        return HashMap::default();
    }

    KMedoids::new(k, MAX_ITERATIONS, random, distance_fn).calculate(points)
}

struct KMedoids<'a, P, F> {
    distance_fn: F,
    k: usize,
    max_iterations: usize,
    random: &'a dyn Random,
    phantom_data: PhantomData<P>,
}

impl<'a, P, F> KMedoids<'a, P, F>
where
    P: Point,
    F: Fn(&P, &P) -> f64,
{
    /// Creates a new K-Medoids instance.
    pub fn new(k: usize, max_iterations: usize, random: &'a dyn Random, distance_fn: F) -> Self {
        Self { k, max_iterations, distance_fn, random, phantom_data: Default::default() }
    }

    fn initialize_medoids(&self, data: &[P]) -> Vec<P> {
        let mut medoids = Vec::with_capacity(self.k);

        // choose first medoid randomly
        let idx = self.random.uniform_int(0, data.len() as i32 - 1);
        medoids.push(data[idx as usize].clone());

        while medoids.len() < self.k {
            // calculate the distance from each point to the nearest medoid
            let distances: Vec<f64> = data
                .iter()
                .map(|point| {
                    medoids
                        .iter()
                        .map(|medoid| (self.distance_fn)(point, medoid))
                        .min_by(|a, b| a.total_cmp(b))
                        .unwrap()
                })
                .collect();

            // choose the next medoid with a probability proportional to the square of the distance
            let total_distance: f64 = distances.iter().sum();
            let cumulative_probabilities = distances
                .iter()
                .scan(0.0, |acc, &dist| {
                    *acc += dist / total_distance;
                    Some(*acc)
                })
                .collect::<Vec<_>>();

            let random_value = self.random.uniform_real(0., 1.);
            let next_medoid_index = cumulative_probabilities.iter().position(|&prob| prob >= random_value).unwrap();

            medoids.push(data[next_medoid_index].clone());
        }

        medoids
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
        let mut medoids = self.initialize_medoids(data);

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
