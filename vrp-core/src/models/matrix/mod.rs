//! Contains functionality to work with solution represented in adjacency matrix from.
//!
//! This is experimental functionality with a main purpose to design more sophisticated
//! metaheuristics which capable to produce *unfeasible solutions and convert them to feasible.
//!
//! *unfeasible solution is solution which has at least one violation of hard constraint.
//!
//!
//! Encoding schema:
//!
//! For each job in plan create a tuple:
//!  single -> places -> times : (job, 0, place_index, time_window_index)
//!  multi -> singles-> places-> times -> (job, single_index, place_index, time_window_index)
//!   => assign unique index
//!
//! For each actor in fleet create a tuple:
//!  actors -> (start, time), (end, time) -> unique
//!  => assign unique index (agreed indexing within jobs)
//!
//! Example:
//!
//! from problem:
//!   actors:       a b c
//!   activities:   (01) 02 03 04 05 06 07 08 09 (10)
//! where (01) and (10) - depots (start and end)
//!
//! routes with their activities in solution:
//!   a: 01 03 06 08 10
//!   b: 01 07 04 10
//!   c: 01 09 05 02 10
//!
//! adjacency matrix:
//!   01 02 03 04 05 06 07 08 09 10
//! 01       a           b     c
//! 02                            c
//! 03                a
//! 04                            b
//! 05    c
//! 06                      a
//! 07          b
//! 08                            a
//! 09             c
//! 10
//!

/// An adjacency matrix trait specifies behaviour of a data structure which is used to store VRP solution.
pub trait AdjacencyMatrix {
    /// Creates a new AdjacencyMatrix with `size`*`size`
    fn new(size: usize) -> Self;

    /// Iterates over unique matrix values.
    fn values<'a>(&'a self) -> Box<dyn Iterator<Item = f64> + 'a>;

    /// Sets given value to cell.
    fn set_cell(&mut self, row: usize, col: usize, value: f64);

    /// Scans given row in order to find first occurrence of element for which predicate returns true.
    fn scan_row<F>(&self, row: usize, predicate: F) -> Option<usize>
    where
        F: Fn(f64) -> bool;
}

mod sparse_matrix;
pub use self::sparse_matrix::*;

mod decipher;
pub use self::decipher::AdjacencyMatrixDecipher;
