#[cfg(test)]
#[path = "../../../tests/unit/models/common/costs_test.rs"]
mod costs_test;

/// Specifies location type.
pub type Cost = f64;

/// Represents actual cost and penalty.
pub struct ObjectiveCost {
    pub actual: Cost,
    pub penalty: Cost,
}

impl ObjectiveCost {
    pub fn new(actual: Cost, penalty: Cost) -> Self {
        Self { actual, penalty }
    }

    /// Returns total cost.
    pub fn total(&self) -> Cost {
        self.actual + self.penalty
    }
}
