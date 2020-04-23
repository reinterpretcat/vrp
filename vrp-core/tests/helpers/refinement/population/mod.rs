// Our multi-variate fitness/solution value
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Tuple(pub usize, pub usize);

mod dominance;
pub use self::dominance::*;

mod objective;
pub use self::objective::*;
