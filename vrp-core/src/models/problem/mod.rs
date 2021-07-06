//! Problem domain models.

use crate::algorithms::nsga2::Objective;
use crate::construction::constraints::ConstraintModule;
use crate::construction::heuristics::InsertionContext;
use std::sync::Arc;

mod costs;
pub use self::costs::*;

mod jobs;
pub use self::jobs::*;

mod fleet;
pub use self::fleet::*;

/// An actual objective on solution type.
pub type TargetObjective = Arc<dyn Objective<Solution = InsertionContext> + Send + Sync>;

/// An actual constraint.
pub type TargetConstraint = Arc<dyn ConstraintModule + Send + Sync>;
