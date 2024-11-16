#[cfg(test)]
#[path = "../../tests/unit/common/routing_test.rs"]
mod routing_test;

use std::sync::Arc;
use vrp_core::custom_extra_property;
use vrp_core::models::common::{Distance, Duration, Location, Profile};
use vrp_core::models::problem::{TransportCost, TravelTime};
use vrp_core::models::solution::Route;
use vrp_core::models::Extras;
use vrp_core::prelude::{GenericError, InfoLogger};
use vrp_core::utils::{Float, GenericResult, Timer};

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

    /// Creates transport (fleet index).
    pub fn create_transport(
        &self,
        is_rounded: bool,
        logger: &InfoLogger,
    ) -> Result<Arc<dyn TransportCost>, GenericError> {
        Timer::measure_duration_with_callback(
            || {
                // NOTE changing to calculating just an upper/lower triangle of the matrix won't improve
                // performance. I think it is related to the fact that we have to change a memory access
                // pattern to less effective one.
                let mut matrix_values = self
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

                matrix_values.shrink_to_fit();

                let transport: Arc<dyn TransportCost> = Arc::new(SingleDataTransportCost::new(matrix_values)?);

                Ok(transport)
            },
            |duration| (logger)(format!("fleet index created in {}ms", duration.as_millis()).as_str()),
        )
    }
}

/// Represents a transport cost which has the same distances as durations and single profile.
struct SingleDataTransportCost {
    size: usize,
    values: Vec<Float>,
}

impl SingleDataTransportCost {
    pub fn new(values: Vec<Float>) -> GenericResult<Self> {
        let size = (values.len() as Float).sqrt() as usize;

        if size * size != values.len() {
            return Err(GenericError::from(format!("non-square flatten matrix: {} items", values.len())));
        }

        Ok(Self { size, values })
    }
}

impl TransportCost for SingleDataTransportCost {
    fn duration_approx(&self, _: &Profile, from: Location, to: Location) -> Duration {
        self.values[from * self.size + to]
    }

    fn distance_approx(&self, _: &Profile, from: Location, to: Location) -> Distance {
        self.values[from * self.size + to]
    }

    fn duration(&self, _: &Route, from: Location, to: Location, _: TravelTime) -> Duration {
        self.values[from * self.size + to]
    }

    fn distance(&self, _: &Route, from: Location, to: Location, _: TravelTime) -> Distance {
        self.values[from * self.size + to]
    }

    fn size(&self) -> usize {
        self.size
    }
}
