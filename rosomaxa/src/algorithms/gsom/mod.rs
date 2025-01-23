//! Provides customized implementation of Growing Self Organizing Map.

use crate::utils::Float;
use std::borrow::Borrow;
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
    fn weights(&self) -> &[Float];
}

/// Represents input data storage.
pub trait Storage: Display + Send + Sync {
    /// An input type.
    type Item: Input;

    /// Adds an input to the storage.
    fn add(&mut self, input: Self::Item);

    /// Returns iterator over all data.
    fn iter(&self) -> Box<dyn Iterator<Item = &'_ Self::Item> + '_>;

    /// Removes and returns all data from the storage.
    fn drain<R>(&mut self, range: R) -> Vec<Self::Item>
    where
        R: RangeBounds<usize>;

    /// Returns a distance between two input weights.
    fn distance<IA, IB>(&self, a: IA, b: IB) -> Float
    where
        IA: Iterator,
        IB: Iterator,
        IA::Item: Borrow<Float>,
        IB::Item: Borrow<Float>;

    /// Shrinks the storage to the specified size.
    fn resize(&mut self, size: usize);

    /// Returns size of the storage.
    fn size(&self) -> usize;
}

/// Represents a storage factory.
pub trait StorageFactory<C, I, S>: Send + Sync
where
    C: Send + Sync,
    I: Input,
    S: Storage<Item = I>,
{
    /// Returns a new storage.
    fn eval(&self, context: &C) -> S;
}
