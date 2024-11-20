#[cfg(test)]
#[path = "../../../tests/unit/algorithms/gsom/node_test.rs"]
mod node_test;

use super::*;
use crate::utils::Float;
use std::collections::VecDeque;
use std::fmt::Formatter;

/// Represents a node in network.
pub struct Node<I: Input, S: Storage<Item = I>> {
    /// A weight vector.
    pub weights: Vec<Float>,
    /// An error of the neuron.
    pub error: Float,
    /// Tracks amount of times node is selected as BU.
    pub total_hits: usize,
    /// Tracks last hits,
    pub last_hits: VecDeque<usize>,
    /// A coordinate in network.
    pub coordinate: Coordinate,
    /// Remembers passed data.
    pub storage: S,
    /// How many last hits should be remembered.
    hit_memory_size: usize,
}

/// Coordinate of the node.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct Coordinate(pub i32, pub i32);

impl<I: Input, S: Storage<Item = I>> Node<I, S> {
    /// Creates a new instance of `Node`.
    pub fn new(coordinate: Coordinate, weights: &[Float], error: Float, hit_memory_size: usize, storage: S) -> Self {
        Self {
            weights: weights.to_vec(),
            error,
            total_hits: 0,
            last_hits: VecDeque::with_capacity(hit_memory_size + 1),
            coordinate,
            storage,
            hit_memory_size,
        }
    }

    /// Adjusts the weights of the node.
    pub fn adjust(&mut self, target: &[Float], learning_rate: Float) {
        debug_assert!(self.weights.len() == target.len());

        for (idx, value) in target.iter().enumerate() {
            self.weights[idx] += learning_rate * (*value - self.weights[idx]);
        }
    }

    /// Returns distance to the given weights.
    pub fn distance(&self, weights: &[Float]) -> Float {
        self.storage.distance(self.weights.as_slice(), weights)
    }

    /// Updates hit statistics.
    pub fn new_hit(&mut self, time: usize) {
        self.total_hits += 1;
        if self.last_hits.front().map_or(true, |last_time| *last_time != time) {
            self.last_hits.push_front(time);
            self.last_hits.truncate(self.hit_memory_size);
        }
    }

    /// Returns amount of last hits.
    pub fn get_last_hits(&self, current_time: usize) -> usize {
        self.last_hits
            .iter()
            .filter(|&hit| {
                if current_time > self.hit_memory_size {
                    (current_time - self.hit_memory_size) < *hit
                } else {
                    true
                }
            })
            .count()
    }

    /// Checks if the cell is at the boundary of the network.
    pub fn is_boundary<C, F>(&self, network: &Network<C, I, S, F>) -> bool
    where
        C: Send + Sync,
        F: StorageFactory<C, I, S>,
    {
        self.neighbours(network, 1).filter(|(_, (x, y))| x.abs() + y.abs() < 2).any(|(node, _)| node.is_none())
    }

    /// Gets iterator over node coordinates in neighbourhood.
    /// If neighbour is not found, then None is returned for corresponding coordinate.
    pub fn neighbours<'a, C, F>(
        &self,
        network: &'a Network<C, I, S, F>,
        radius: usize,
    ) -> impl Iterator<Item = (Option<Coordinate>, (i32, i32))> + 'a
    where
        C: Send + Sync,
        F: StorageFactory<C, I, S>,
    {
        let radius = radius as i32;
        let Coordinate(node_x, node_y) = self.coordinate;

        (-radius..=radius).flat_map(move |x| {
            (-radius..=radius)
                .filter(move |&y| !(x == 0 && y == 0))
                .map(move |y| (network.find(&Coordinate(node_x + x, node_y + y)).map(|node| node.coordinate), (x, y)))
        })
    }

    /// Gets unified distance.
    pub fn unified_distance<C, F>(&self, network: &Network<C, I, S, F>, radius: usize) -> Float
    where
        C: Send + Sync,
        F: StorageFactory<C, I, S>,
    {
        let (sum, count) = self
            .neighbours(network, radius)
            .filter_map(|(coord, _)| coord.and_then(|coord| network.find(&coord)))
            .fold((0., 0), |(sum, count), node| {
                let distance = self.storage.distance(self.weights.as_slice(), node.weights.as_slice());
                (sum + distance, count + 1)
            });

        if count > 0 {
            sum / count as Float
        } else {
            0.
        }
    }

    /// Returns distance between underlying item (if any) and node weight's.
    pub fn node_distance(&self) -> Option<Float> {
        self.storage.iter().next().map(|item| self.storage.distance(self.weights.as_slice(), item.weights()))
    }

    /// Calculates mean squared error of the node.
    pub fn mse(&self) -> Float {
        let (count, sum) = self
            .storage
            .iter()
            // NOTE try only first item so far
            .take(1)
            .fold((0, 0.), |(items, acc), data| {
                let err = data
                    .weights()
                    .iter()
                    .zip(self.weights.iter())
                    .map(|(&w1, &w2)| (w1 - w2) * (w1 - w2))
                    .sum::<Float>()
                    / self.weights.len() as Float;

                (items + 1, acc + err)
            });

        if count > 0 {
            sum / count as Float
        } else {
            sum
        }
    }
}

impl Display for Coordinate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("({},{})", self.0, self.1))
    }
}

impl From<(i32, i32)> for Coordinate {
    fn from(value: (i32, i32)) -> Self {
        Coordinate(value.0, value.1)
    }
}
