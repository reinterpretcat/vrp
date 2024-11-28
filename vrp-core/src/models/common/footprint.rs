//! This module contains a logic to maintain a low-dimensional representation of the VRP Solution.

#[cfg(test)]
#[path = "../../../tests/unit/models/common/footprint_test.rs"]
mod footprint_test;

use crate::algorithms::structures::BitVec;
use crate::models::common::Location;
use crate::prelude::*;
use rosomaxa::population::RosomaxaContext;
use rosomaxa::utils::fold_reduce;

custom_solution_state!(Footprint typeof Footprint);

/// Defines a low-dimensional representation of multiple solutions.
#[derive(Clone, Debug)]
pub struct Footprint {
    /// repr here is the same adjacency matrix as in [Shadow], but instead of storing bits
    /// we store here the number of times the edge was present in multiple solutions.
    repr: Vec<u8>,
    dimension: usize,
    counter: usize,
}

impl Footprint {
    /// Creates a new instance of a `Snapshot`.
    pub fn new(problem: &Problem) -> Self {
        let dim = get_dimension(problem);
        Self { repr: vec![0; dim * dim], dimension: dim, counter: 0 }
    }

    /// Adds shadow to the footprint.
    pub fn add(&mut self, shadow: &Shadow) {
        self.repr.iter_mut().enumerate().for_each(|(index, value)| {
            let bit_value = shadow.repr.get(index).map(|bit| bit as u8).unwrap_or_default();
            *value = value.saturating_add(bit_value);
        });
    }

    /// Merges/unites the given footprint into the current one.
    pub fn union(&mut self, other: &Footprint) {
        self.repr.iter_mut().zip(other.repr.iter()).for_each(|(value, other_value)| {
            *value = value.saturating_add(*other_value);
        });
    }

    /// Returns the number of times the edge was present in multiple solutions.
    pub fn get(&self, from: usize, to: usize) -> u8 {
        let from = from % self.dimension;
        let to = to % self.dimension;

        self.repr[from * self.dimension + to]
    }

    /// Returns an iterator over the low-dimensional representation.
    pub fn iter(&self) -> impl Iterator<Item = ((usize, usize), u8)> + '_ {
        (0..self.dimension).flat_map(move |from| (0..self.dimension).map(move |to| ((from, to), self.get(from, to))))
    }

    /// Reshapes the footprint to keep sensitivity to new solutions.
    pub fn forget(&mut self) {
        self.repr.iter_mut().for_each(|value| {
            // NOTE use log2 to reduce the impact of the number of times the edge was present in multiple solutions.
            *value = (*value as f64).log2() as u8;
        });
    }

    /// Estimates the cost of a solution as a sum of all edge costs.
    pub fn estimate_solution(&self, solution_ctx: &SolutionContext) -> usize {
        solution_ctx
            .routes
            .iter()
            .flat_map(|route_ctx| {
                route_ctx
                    .route()
                    .tour
                    .legs()
                    .filter_map(|(activities, _)| if let [from, to] = activities { Some((from, to)) } else { None })
                    .map(|(from, to)| (from.place.location, to.place.location))
                    .map(|(from, to)| self.get(from, to) as usize)
            })
            .sum()
    }

    /// Estimates how frequently the edge between two locations was present in multiple solutions.
    pub fn estimate_edge(&self, from: Location, to: Location) -> usize {
        self.get(from, to) as usize
    }

    /// Returns dimension of adjacency matrix.
    pub fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Specifies a maximum number of locations considered in the low-dimensional representation.
const MAX_REPRESENTATION_DIMENSION: usize = 256;

/// Specifies a rate at which the footprint scales down the number of times the edge was present in multiple solutions.
const FOOTPRINT_FORGET_RATE: usize = 200;

/// A low-dimensional representation of the VRP Solution.
/// Here, we use Bit Vector data structure to represent the adjacency matrix of the solution, where
/// each bit represents the presence of the edge between pair of locations in the given solution.
#[derive(Clone, Debug)]
pub struct Shadow {
    /// repr is adjusted matrix of size dim x dim, where dim is the minimum of MAX_REPRESENTATION_DIMENSION
    /// and number of locations present in the problem.
    repr: BitVec,
}

impl Shadow {
    /// Returns an iterator over the low-dimensional representation.
    pub fn iter(&self) -> impl Iterator<Item = ((usize, usize), bool)> + '_ {
        let size = (self.repr.len() as f64).sqrt() as usize;
        debug_assert!(size * size == self.repr.len());

        (0..size).flat_map(move |from| (0..size).map(move |to| ((from, to), self.repr[from * size + to])))
    }

    /// Returns dimension of the shadow.
    pub fn dimension(&self) -> usize {
        (self.repr.len() as f64).sqrt() as usize
    }
}

impl From<&InsertionContext> for Shadow {
    fn from(insertion_ctx: &InsertionContext) -> Self {
        let dim = get_dimension(&insertion_ctx.problem);
        let mut shadow = Shadow { repr: BitVec::new(dim * dim) };

        insertion_ctx.solution.routes.iter().for_each(|route_ctx| {
            route_ctx
                .route()
                .tour
                .legs()
                .filter_map(|(activities, _)| if let [from, to] = activities { Some((from, to)) } else { None })
                // NOTE apply % operator on locations. This is not optimal, but it is the simplest and
                // the fastest approach to keep memory usage quite low. Better, but slower approach would be
                // to apply some clustering algorithm for nearby locations and use the same index to them.
                .map(|(from, to)| (from.place.location % dim, to.place.location % dim))
                .for_each(|(from, to)| shadow.repr.set(from * dim + to, true));
        });

        shadow
    }
}

impl RosomaxaContext for Footprint {
    type Solution = InsertionContext;

    fn on_change(&mut self, solutions: &[Self::Solution]) {
        if solutions.is_empty() {
            return;
        }

        // we need to forget the footprint from time to time to keep it sensitive to new solutions
        if self.counter == FOOTPRINT_FORGET_RATE {
            self.forget();
            self.counter = 0;
        } else {
            self.counter += 1;
        }

        // merge solutions into the footprint in parallel from performance reasons
        let problem = solutions[0].problem.clone();
        let footprint = fold_reduce(
            solutions,
            || Footprint::new(&problem),
            |mut footprint, solution| {
                footprint.add(&Shadow::from(solution));
                footprint
            },
            |mut left, right| {
                left.union(&right);
                left
            },
        );
        self.union(&footprint);
    }
}

fn get_dimension(problem: &Problem) -> usize {
    problem.transport.size().min(MAX_REPRESENTATION_DIMENSION)
}
