//! Specifies logic to create a "pragmatic" solution and write it into json format.

mod model;
pub use self::model::*;

pub(crate) mod activity_matcher;

mod geo_serializer;
pub use self::geo_serializer::serialize_solution_as_geojson;

mod initial_reader;
pub use self::initial_reader::read_init_solution;

mod extensions;

mod writer;
pub use self::writer::create_solution;
pub use self::writer::PragmaticSolution;
