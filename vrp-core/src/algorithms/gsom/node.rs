use super::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Represents a node in network.
pub struct Node<I: Input, S: Storage<Item = I>> {
    /// A weight vector.
    pub weights: Vec<f64>,
    /// An error of the neuron.
    pub error: f64,
    /// A coordinate in network.
    pub coordinate: Coordinate,
    /// A reference to topology.
    pub topology: Topology<I, S>,
    /// Remembers passed data.
    pub storage: S,
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

pub type NodeLink<I, S> = Rc<RefCell<Node<I, S>>>;

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Coordinate(pub i32, pub i32);

impl<I: Input, S: Storage<Item = I>> Node<I, S> {
    /// Creates a new instance of `Node`.
    pub fn new(coordinate: Coordinate, weights: &[f64]) -> Self {
        Self {
            weights: weights.to_vec(),
            error: 0.0,
            coordinate,
            topology: Topology::empty(weights.len()),
            storage: S::default(),
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
}

impl<I: Input, S: Storage<Item = I>> Topology<I, S> {
    /// Creates an empty cell at given coordinate.
    pub fn empty(dimension: usize) -> Self {
        Self { dimension, right: None, left: None, up: None, down: None }
    }

    /// Gets neighbors.
    pub fn neighbours<'a>(&'a self) -> impl Iterator<Item = &'a NodeLink<I, S>> + 'a {
        TopologyIterator { topology: self, state: 0 }
    }

    /// Checks if the cell is at the boundary of the network.
    pub fn is_boundary(&self) -> bool {
        self.right.is_none() || self.left.is_none() || self.up.is_none() || self.down.is_none()
    }
}

struct TopologyIterator<'a, I: Input, S: Storage<Item = I>> {
    topology: &'a Topology<I, S>,
    state: usize,
}

impl<'a, I: Input, S: Storage<Item = I>> TopologyIterator<'a, I, S> {
    fn transition(&mut self, state: usize, node: Option<&'a NodeLink<I, S>>) -> Result<(), Option<&'a NodeLink<I, S>>> {
        if self.state == state {
            self.state = state + 1;
            if node.is_some() {
                return Err(node);
            }
        }

        Ok(())
    }

    fn iterate(&mut self) -> Result<(), Option<&'a NodeLink<I, S>>> {
        self.transition(0, self.topology.left.as_ref())?;
        self.transition(1, self.topology.right.as_ref())?;
        self.transition(2, self.topology.down.as_ref())?;
        self.transition(3, self.topology.up.as_ref())?;

        Ok(())
    }
}

impl<'a, I: Input, S: Storage<Item = I>> Iterator for TopologyIterator<'a, I, S> {
    type Item = &'a NodeLink<I, S>;

    fn next(&mut self) -> Option<&'a NodeLink<I, S>> {
        if let Err(node) = self.iterate() {
            node
        } else {
            None
        }
    }
}
