use std::sync::Arc;
use vrp_core::models::common::Location;
use vrp_core::models::problem::{create_matrix_transport_cost, MatrixData, TransportCost};

pub struct MatrixFactory {
    locations: Vec<(i32, i32)>,
}

impl Default for MatrixFactory {
    fn default() -> Self {
        Self { locations: vec![] }
    }
}

impl MatrixFactory {
    pub fn collect(&mut self, location: (i32, i32)) -> Location {
        match self.locations.iter().position(|l| l.0 == location.0 && l.1 == location.1) {
            Some(position) => position,
            _ => {
                self.locations.push(location);
                self.locations.len() - 1
            }
        }
    }

    pub fn create_transport(&self) -> Result<Arc<dyn TransportCost + Send + Sync>, String> {
        let matrix_values = self
            .locations
            .iter()
            .flat_map(|&(x1, y1)| {
                self.locations.iter().map(move |&(x2, y2)| {
                    let x = x1 as f64 - x2 as f64;
                    let y = y1 as f64 - y2 as f64;
                    (x * x + y * y).sqrt()
                })
            })
            .collect::<Vec<f64>>();

        let matrix_data = MatrixData::new(0, None, matrix_values.clone(), matrix_values);

        create_matrix_transport_cost(vec![matrix_data])
    }
}
