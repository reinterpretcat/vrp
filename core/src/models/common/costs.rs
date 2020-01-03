#[cfg(test)]
#[path = "../../../tests/unit/models/common/costs_test.rs"]
mod costs_test;

/// Specifies cost value.
pub type Cost = f64;

/// Represents actual cost and penalty used by objective function.
#[derive(Clone)]
pub struct ObjectiveCost {
    /// Actual cost of solution without penalties.
    pub actual: Cost,
    /// Penalty cost.
    pub penalty: Cost,
}

impl ObjectiveCost {
    /// Creates a new [`ObjectiveCost`]
    pub fn new(actual: Cost, penalty: Cost) -> Self {
        Self { actual, penalty }
    }

    /// Returns total cost.
    pub fn total(&self) -> Cost {
        self.actual + self.penalty
    }
}
