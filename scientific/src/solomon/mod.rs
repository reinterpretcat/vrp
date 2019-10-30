mod reader;
pub use self::reader::read_solomon_format;
pub use self::reader::SolomonProblem;

mod writer;
pub use self::writer::write_solomon_solution;
