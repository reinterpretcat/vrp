// Our multi-variate fitness/solution value
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Tuple(pub usize, pub usize);

mod objective;
pub use self::objective::*;
