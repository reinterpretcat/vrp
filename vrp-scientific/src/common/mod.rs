//! Contains common text reading and writing functionality.

mod text_reader;
pub use self::text_reader::create_dimens_with_id;
pub use self::text_reader::create_fleet_with_distance_costs;
pub use self::text_reader::read_init_solution;
pub use self::text_reader::read_line;
pub use self::text_reader::StringReader;
pub use self::text_reader::TextReader;

mod text_writer;
pub use self::text_writer::write_text_solution;
