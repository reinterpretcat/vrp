//! A solver crate contains metaheuristic implementation to solve arbitrary VRP problem.

mod algorithm;
pub use self::algorithm::Solver;

mod builder;
pub use self::builder::SolverBuilder;

mod extensions;
