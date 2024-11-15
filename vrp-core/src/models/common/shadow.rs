//! This module contains a logic to maintain a low-dimensional representation of the VRP Solution.

use crate::algorithms::structures::BitVec;
use crate::prelude::*;

/// A maximum size of locations.
const MAX_REPR_BIT_VEC_SIZE: usize = 1000;

// A state property to store the low-dimensional representation of the solution.
custom_solution_state!(Shadow typeof Shadow);

/// A low-dimensional representation of the VRP Solution.
/// Here, we use Bit Vector data structure to represent the adjacency matrix of the solution, where
/// each bit represents the presence of the edge between pair of locations in the given solution.
pub(crate) struct Shadow {
    repr: BitVec,
}

impl From<&InsertionContext> for Shadow {
    fn from(insertion_ctx: &InsertionContext) -> Self {
        let size = insertion_ctx.problem.transport.size().min(MAX_REPR_BIT_VEC_SIZE);
        let mut shadow = Shadow { repr: BitVec::new(size * size) };

        insertion_ctx.solution.routes.iter().for_each(|route_ctx| {
            route_ctx
                .route()
                .tour
                .legs()
                .filter_map(|(activities, _)| if let [from, to] = activities { Some((from, to)) } else { None })
                // NOTE apply % operator on locations. This is not optimal, but it is the simplest and
                // the fastest approach to keep memory usage quite low. Better, but slower approach would be
                // to apply some clustering algorithm for nearby locations and use the same index to them.
                .map(|(from, to)| (from.place.location % size, to.place.location % size))
                .for_each(|(from, to)| shadow.repr.set(from * size + to, true));
        });

        shadow
    }
}
