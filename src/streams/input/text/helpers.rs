use crate::construction::constraints::*;
use crate::models::common::*;
use crate::models::problem::*;
use std::sync::Arc;

pub fn create_fleet_with_distance_costs(number: usize, capacity: usize, location: Location, time: TimeWindow) -> Fleet {
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

pub fn create_dimens_with_id(prefix: &str, id: usize) -> Dimensions {
    let mut dimens = Dimensions::new();
    dimens.set_id([prefix.to_string(), id.to_string()].concat().as_str());
    dimens
}

pub fn create_constraint(activity: Arc<SimpleActivityCost>, transport: Arc<MatrixTransportCost>) -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::new();
    constraint.add_module(Box::new(TimingConstraintModule::new(activity, transport, 1)));
    constraint.add_module(Box::new(CapacityConstraintModule::<i32>::new(2)));

    constraint
}
