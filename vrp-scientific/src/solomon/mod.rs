//! Contains functionality to read solomon problem and write its solution.

mod initial_reader;
pub use self::initial_reader::read_init_solution;

mod reader;
pub use self::reader::SolomonProblem;

mod writer;
pub use self::writer::SolomonSolution;
