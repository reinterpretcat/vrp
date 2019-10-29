use crate::models::common::Location;
use crate::models::problem::MatrixTransportCost;

pub struct MatrixFactory {
    locations: Vec<(i32, i32)>,
}

impl MatrixFactory {
    pub fn new() -> Self {
        Self { locations: vec![] }
    }

    pub fn collect(&mut self, location: (i32, i32)) -> Location {
        match self.locations.iter().position(|l| l.0 == location.0 && l.1 == location.1) {
            Some(position) => position,
            _ => {
                self.locations.push(location);
                self.locations.len() - 1
            }
        }
    }

    pub fn create_transport(&self) -> MatrixTransportCost {
        let matrix_data = self
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

        MatrixTransportCost::new(vec![matrix_data.clone()], vec![matrix_data])
    }
}
