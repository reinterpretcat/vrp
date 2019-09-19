mod comparison;

pub use self::comparison::compare_floats;
pub use self::comparison::compare_shared;

mod permutations;
pub use self::permutations::get_permutations;
pub use self::permutations::Permutations;
