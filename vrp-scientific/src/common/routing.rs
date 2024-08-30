#[cfg(test)]
#[path = "../../tests/unit/common/routing_test.rs"]
mod routing_test;

use std::sync::Arc;
use vrp_core::custom_extra_property;
use vrp_core::models::common::Location;
use vrp_core::models::problem::{create_matrix_transport_cost, MatrixData, TransportCost};
use vrp_core::models::Extras;
use vrp_core::prelude::GenericError;
use vrp_core::utils::Float;

custom_extra_property!(CoordIndex typeof CoordIndex);

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
    pub fn create_transport(&self, is_rounded: bool) -> Result<Arc<dyn TransportCost>, GenericError> {
        let matrix_values = self
            .locations
            .iter()
            .flat_map(|&(x1, y1)| {
                self.locations.iter().map(move |&(x2, y2)| {
                    let x = x1 as Float - x2 as Float;
                    let y = y1 as Float - y2 as Float;
                    let value = (x * x + y * y).sqrt();

                    if is_rounded {
                        value.round()
                    } else {
                        value
                    }
                })
            })
            .collect::<Vec<Float>>();

        let matrix_data = MatrixData::new(0, None, matrix_values.clone(), matrix_values);

        create_matrix_transport_cost(vec![matrix_data])
    }
}
