#[cfg(test)]
#[path = "../../../tests/unit/algorithms/gsom/node_test.rs"]
mod node_test;

use super::*;
use std::collections::VecDeque;
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
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
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
            topology: Topology::empty(),
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
        Self { right: self.right.clone(), left: self.left.clone(), up: self.up.clone(), down: self.down.clone() }
    }
}

impl<I: Input, S: Storage<Item = I>> Topology<I, S> {
    /// Creates an empty cell at given coordinate.
    pub fn empty() -> Self {
        Self { right: None, left: None, up: None, down: None }
    }

    /// Checks if the cell is at the boundary of the network.
    pub fn is_boundary(&self) -> bool {
        self.right.is_none() || self.left.is_none() || self.up.is_none() || self.down.is_none()
    }

    /// Gets iterator over nodes in neighbourhood.
    pub fn neighbours(&self, radius: usize) -> impl Iterator<Item = (Option<NodeLink<I, S>>, Coordinate)> {
        let extras = match radius {
            1 => vec![],
            2 => vec![
                (self.left.as_ref().and_then(|node| node.read().unwrap().topology.up.clone()), Coordinate(-1, 1)),
                (self.right.as_ref().and_then(|node| node.read().unwrap().topology.up.clone()), Coordinate(1, 1)),
                (self.left.as_ref().and_then(|node| node.read().unwrap().topology.down.clone()), Coordinate(-1, -1)),
                (self.right.as_ref().and_then(|node| node.read().unwrap().topology.down.clone()), Coordinate(1, -1)),
            ],
            _ => unimplemented!("neighbourhood radius is supported only in [1, 2] range"),
        };

        TopologyIterator { topology: self.clone(), state: 0, extras }
    }
}

struct TopologyIterator<I: Input, S: Storage<Item = I>> {
    topology: Topology<I, S>,
    state: usize,
    extras: Vec<(Option<NodeLink<I, S>>, Coordinate)>,
}

impl<I: Input, S: Storage<Item = I>> Iterator for TopologyIterator<I, S> {
    type Item = (Option<NodeLink<I, S>>, Coordinate);

    fn next(&mut self) -> Option<Self::Item> {
        let ext_len = self.extras.len();

        debug_assert!(ext_len == 0 || ext_len == 4);

        let item = match self.state {
            0 => (self.topology.left.clone(), Coordinate(-1, 0)),
            1 => (self.topology.right.clone(), Coordinate(1, 0)),
            2 => (self.topology.up.clone(), Coordinate(0, 1)),
            3 => (self.topology.down.clone(), Coordinate(0, -1)),
            state if ext_len > (state - ext_len) => self.extras.get(state - ext_len).cloned().unwrap(),
            _ => return None,
        };

        self.state += 1;

        Some(item)
    }
}
