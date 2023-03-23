//! Provides customized implementation of Growing Self Organizing Map.

use std::fmt::Display;
use std::ops::RangeBounds;

mod contraction;
pub(crate) use self::contraction::*;

mod network;
pub use self::network::*;

mod node;
pub use self::node::*;

mod state;
pub use self::state::*;

/// Represents an input for network.
pub trait Input: Send + Sync {
    /// Returns weights.
    fn weights(&self) -> &[f64];
}

/// Represents input data storage.
pub trait Storage: Display + Send + Sync {
    /// An input type.
    type Item: Input;

    /// Adds an input to the storage.
    fn add(&mut self, input: Self::Item);

    /// Returns iterator over all data.
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Item> + 'a>;

    /// Removes and returns all data from the storage.
    fn drain<R>(&mut self, range: R) -> Vec<Self::Item>
    where
        R: RangeBounds<usize>;

    /// Returns a distance between two input weights.
    fn distance(&self, a: &[f64], b: &[f64]) -> f64;

    /// Returns size of the storage.
    fn size(&self) -> usize;
}

/// Represents a storage factory.
pub trait StorageFactory<I, S>: Send + Sync
where
    I: Input,
    S: Storage<Item = I>,
{
    /// Returns a new storage.
    fn eval(&self) -> S;
}
