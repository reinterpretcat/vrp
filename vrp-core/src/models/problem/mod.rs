//! Problem domain models.

use crate::construction::constraints::ConstraintModule;
use crate::construction::heuristics::InsertionContext;
use crate::models::common::Objective;

mod costs;
pub use self::costs::*;

mod jobs;
pub use self::jobs::*;

mod fleet;
pub use self::fleet::*;

/// An actual objective on solution type.
pub type TargetObjective = Box<dyn Objective<Solution = InsertionContext> + Send + Sync>;

/// An actual constraint.
pub type TargetConstraint = Box<dyn ConstraintModule + Send + Sync>;
