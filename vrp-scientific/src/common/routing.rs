#[cfg(test)]
#[path = "../../tests/unit/common/routing_test.rs"]
mod routing_test;

use std::sync::Arc;
use vrp_core::construction::constraints::NoTravelLimits;
use vrp_core::models::common::Location;
use vrp_core::models::problem::{create_matrix_transport_cost, MatrixData, TransportCost};

/// Represents a coord index which can be used to analyze customer's locations.
#[derive(Clone, Default)]
pub struct CoordIndex {
    /// Keeps track of locations.
    pub locations: Vec<(i32, i32)>,
}

impl CoordIndex {
    /// Adds location to index.
    pub fn collect(&mut self, location: (i32, i32)) -> Location {
        match self.locations.iter().position(|l| l.0 == location.0 && l.1 == location.1) {
            Some(position) => position,
            _ => {
                self.locations.push(location);
                self.locations.len() - 1
            }
        }
    }

    /// Creates transport.
    pub fn create_transport(&self, is_rounded: bool) -> Result<Arc<dyn TransportCost + Send + Sync>, String> {
        let matrix_values = self
            .locations
            .iter()
            .flat_map(|&(x1, y1)| {
                self.locations.iter().map(move |&(x2, y2)| {
                    let x = x1 as f64 - x2 as f64;
                    let y = y1 as f64 - y2 as f64;
                    let value = (x * x + y * y).sqrt();

                    if is_rounded {
                        value.round()
                    } else {
                        value
                    }
                })
            })
            .collect::<Vec<f64>>();

        let matrix_data = MatrixData::new(0, None, matrix_values.clone(), matrix_values);

        create_matrix_transport_cost(vec![matrix_data], Arc::new(NoTravelLimits::default()))
    }
}
