use crate::construction::constraints::*;
use crate::models::common::*;
use crate::models::problem::*;
use std::io::Read;
use std::slice::Iter;
use std::sync::Arc;

pub struct StringReader<'a> {
    iter: Iter<'a, u8>,
}

impl<'a> StringReader<'a> {
    pub fn new(data: &'a str) -> Self {
        Self { iter: data.as_bytes().iter() }
    }
}

impl<'a> Read for StringReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        for i in 0..buf.len() {
            if let Some(x) = self.iter.next() {
                buf[i] = *x;
            } else {
                return Ok(i);
            }
        }
        Ok(buf.len())
    }
}

fn create_fleet_with_distance_costs(number: usize, capacity: usize, location: Location, time: TimeWindow) -> Fleet {
    Fleet::new(
        vec![Driver {
            costs: Costs {
                fixed: 0.0,
                per_distance: 0.0,
                per_driving_time: 0.0,
                per_waiting_time: 0.0,
                per_service_time: 0.0,
            },
            dimens: create_dimens_with_id("driver", 0),
            details: Default::default(),
        }],
        (0..number)
            .map(|i| {
                let mut dimens = create_dimens_with_id("v", i);
                dimens.set_capacity(capacity as i32);
                Vehicle {
                    profile: 0,
                    costs: Costs {
                        fixed: 0.0,
                        per_distance: 1.0,
                        per_driving_time: 0.0,
                        per_waiting_time: 0.0,
                        per_service_time: 0.0,
                    },
                    dimens,
                    details: vec![VehicleDetail {
                        start: Some(location),
                        end: Some(location),
                        time: Some(time.clone()),
                    }],
                }
            })
            .collect(),
    )
}

fn create_dimens_with_id(prefix: &str, id: usize) -> Dimensions {
    let mut dimens = Dimensions::new();
    dimens.set_id([prefix.to_string(), id.to_string()].concat().as_str());
    dimens
}

fn create_constraint(activity: Arc<SimpleActivityCost>, transport: Arc<MatrixTransportCost>) -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::new();
    constraint.add_module(Box::new(TimingConstraintModule::new(activity, transport, 1)));
    constraint.add_module(Box::new(CapacityConstraintModule::<i32>::new(2)));

    constraint
}

mod solomon;
pub use self::solomon::SolomonProblem;

mod lilim;
pub use self::lilim::LilimProblem;
