//! Specifies logic to create a "pragmatic" solution and write it into json format.

mod default_serializer;
pub use self::default_serializer::*;

mod geo_serializer;
pub use self::geo_serializer::serialize_solution_as_geojson;

mod extensions;

mod writer;
pub use self::writer::create_solution;
pub use self::writer::PragmaticSolution;
