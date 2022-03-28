#[cfg(test)]
#[path = "../../../tests/unit/algorithms/gsom/node_test.rs"]
mod node_test;

use super::*;
use std::collections::VecDeque;
use std::iter::once;
use std::sync::{Arc, RwLock};

/// Represents a node in network.
pub struct Node<I: Input, S: Storage<Item = I>> {
    /// A weight vector.
    pub weights: Vec<f64>,
    /// An error of the neuron.
    pub error: f64,
    /// Tracks amount of times node is selected as BU.
    pub total_hits: usize,
    /// Tracks last hits,
    pub last_hits: VecDeque<usize>,
    /// A coordinate in network.
    pub coordinate: Coordinate,
    /// A reference to topology.
    pub topology: Topology<I, S>,
    /// Remembers passed data.
    pub storage: S,
    /// How many last hits should be remembered.
    hit_memory_size: usize,
}

/// Represents a node neighbourhood.
pub struct Topology<I: Input, S: Storage<Item = I>> {
    /// An input dimension.
    pub dimension: usize,
    /// A link to right neighbour.
    pub right: Option<NodeLink<I, S>>,
    /// A link to left neighbour.
    pub left: Option<NodeLink<I, S>>,
    /// A link to up neighbour.
    pub up: Option<NodeLink<I, S>>,
    /// A link to down neighbour.
    pub down: Option<NodeLink<I, S>>,
}

/// A reference to the node.
pub type NodeLink<I, S> = Arc<RwLock<Node<I, S>>>;

/// Coordinate of the node.
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Coordinate(pub i32, pub i32);

impl<I: Input, S: Storage<Item = I>> Node<I, S> {
    /// Creates a new instance of `Node`.
    pub fn new(coordinate: Coordinate, weights: &[f64], error: f64, hit_memory_size: usize, storage: S) -> Self {
        Self {
            weights: weights.to_vec(),
            error,
            total_hits: 0,
            last_hits: VecDeque::with_capacity(hit_memory_size + 1),
            coordinate,
            topology: Topology::empty(weights.len()),
            storage,
            hit_memory_size,
        }
    }

    /// Adjusts the weights of the node.
    pub fn adjust(&mut self, target: &[f64], learning_rate: f64) {
        debug_assert!(self.weights.len() == target.len());

        for (idx, value) in target.iter().enumerate() {
            self.weights[idx] += learning_rate * (*value - self.weights[idx]);
        }
    }

    /// Returns distance to the given weights.
    pub fn distance(&self, weights: &[f64]) -> f64 {
        self.storage.distance(self.weights.as_slice(), weights)
    }

    /// Updates hit statistics.
    pub fn new_hit(&mut self, time: usize) {
        self.total_hits += 1;
        if self.last_hits.get(0).map_or(true, |last_time| *last_time != time) {
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
}

impl<I: Input, S: Storage<Item = I>> Clone for Topology<I, S> {
    fn clone(&self) -> Self {
        Self {
            dimension: self.dimension,
            right: self.right.clone(),
            left: self.left.clone(),
            up: self.up.clone(),
            down: self.down.clone(),
        }
    }
}

impl<I: Input, S: Storage<Item = I>> Topology<I, S> {
    /// Creates an empty cell at given coordinate.
    pub fn empty(dimension: usize) -> Self {
        Self { dimension, right: None, left: None, up: None, down: None }
    }

    /// Checks if the cell is at the boundary of the network.
    pub fn is_boundary(&self) -> bool {
        self.iter().count() < 4
    }

    /// Iterates over non-empty nodes in neighborhood.
    pub fn iter(&self) -> impl Iterator<Item = (&NodeLink<I, S>, Coordinate)> {
        TopologyIterator { topology: self, state: 0 }
    }

    /// Iterates over all neighbour nodes, including, potentially, empty.
    pub fn all(&self) -> impl Iterator<Item = (Option<&NodeLink<I, S>>, Coordinate)> {
        once((self.left.as_ref(), Coordinate(-1, 0)))
            .chain(once((self.right.as_ref(), Coordinate(1, 0))))
            .chain(once((self.up.as_ref(), Coordinate(0, 1))))
            .chain(once((self.down.as_ref(), Coordinate(0, -1))))
    }
}

struct TopologyIterator<'a, I: Input, S: Storage<Item = I>> {
    topology: &'a Topology<I, S>,
    state: usize,
}

impl<'a, I: Input, S: Storage<Item = I>> Iterator for TopologyIterator<'a, I, S> {
    type Item = (&'a NodeLink<I, S>, Coordinate);

    fn next(&mut self) -> Option<Self::Item> {
        let (node, coordinate) = match self.state {
            0 => (self.topology.left.as_ref(), Coordinate(-1, 0)),
            1 => (self.topology.right.as_ref(), Coordinate(1, 0)),
            2 => (self.topology.up.as_ref(), Coordinate(0, 1)),
            3 => (self.topology.down.as_ref(), Coordinate(0, -1)),
            _ => return None,
        };

        self.state += 1;

        if let Some(node) = node {
            Some((node, coordinate))
        } else {
            self.next()
        }
    }
}
