//! Specifies logic to create a "pragmatic" solution and write it into json format.

mod serializer;
pub use self::serializer::*;

mod extensions;

mod writer;
pub use self::writer::create_solution;
pub use self::writer::PragmaticSolution;
