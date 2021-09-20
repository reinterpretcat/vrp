use std::io::prelude::*;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::models::common::*;
use vrp_core::models::problem::*;
use vrp_core::models::{Extras, Problem};

pub(crate) trait TextReader {
    fn read_problem(&mut self, is_rounded: bool) -> Result<Problem, String> {
        let (jobs, fleet) = self.read_definitions()?;
        let transport = self.create_transport(is_rounded)?;
        let activity = Arc::new(SimpleActivityCost::default());
        let jobs = Jobs::new(&fleet, jobs, &transport);

        Ok(Problem {
            fleet: Arc::new(fleet),
            jobs: Arc::new(jobs),
            locks: vec![],
            constraint: Arc::new(create_constraint(activity.clone(), transport.clone())),
            activity,
            transport,
            objective: Arc::new(ObjectiveCost::default()),
            extras: Arc::new(self.create_extras()),
        })
    }

    fn read_definitions(&mut self) -> Result<(Vec<Job>, Fleet), String>;

    fn create_transport(&self, is_rounded: bool) -> Result<Arc<dyn TransportCost + Send + Sync>, String>;

    fn create_extras(&self) -> Extras;
}

pub(crate) fn create_fleet_with_distance_costs(
    number: usize,
    capacity: usize,
    location: Location,
    time: TimeWindow,
) -> Fleet {
    Fleet::new(
        vec![Arc::new(Driver {
            costs: Costs {
                fixed: 0.0,
                per_distance: 0.0,
                per_driving_time: 0.0,
                per_waiting_time: 0.0,
                per_service_time: 0.0,
            },
            dimens: create_dimens_with_id("driver", &0.to_string()),
            details: Default::default(),
        })],
        (0..number)
            .map(|i| {
                let mut dimens = create_dimens_with_id("v", &i.to_string());
                dimens.set_capacity(SingleDimLoad::new(capacity as i32));
                Arc::new(Vehicle {
                    profile: Profile::default(),
                    costs: Costs {
                        fixed: 0.0,
                        per_distance: 1.0,
                        per_driving_time: 0.0,
                        per_waiting_time: 0.0,
                        per_service_time: 0.0,
                    },
                    dimens,
                    details: vec![VehicleDetail {
                        start: Some(VehiclePlace {
                            location,
                            time: TimeInterval { earliest: Some(time.start), latest: None },
                        }),
                        end: Some(VehiclePlace {
                            location,
                            time: TimeInterval { earliest: None, latest: Some(time.end) },
                        }),
                    }],
                })
            })
            .collect(),
        Box::new(|_| Box::new(|_| 0)),
    )
}

pub(crate) fn create_dimens_with_id(prefix: &str, id: &str) -> Dimensions {
    let mut dimens = Dimensions::new();
    dimens.set_id([prefix.to_string(), id.to_string()].concat().as_str());
    dimens
}

pub(crate) fn create_constraint(
    activity: Arc<SimpleActivityCost>,
    transport: Arc<dyn TransportCost + Send + Sync>,
) -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::default();
    constraint.add_module(Arc::new(TransportConstraintModule::new(
        transport.clone(),
        activity,
        Arc::new(|_| (None, None)),
        1,
        2,
        3,
    )));
    constraint.add_module(Arc::new(CapacityConstraintModule::<SingleDimLoad>::new(transport, 4)));
    constraint.add_module(Arc::new(FleetUsageConstraintModule::new_minimized()));

    constraint
}

pub(crate) fn read_line<R: Read>(reader: &mut BufReader<R>, mut buffer: &mut String) -> Result<usize, String> {
    buffer.clear();
    reader.read_line(&mut buffer).map_err(|err| err.to_string())
}

pub(crate) fn skip_lines<R: Read>(count: usize, reader: &mut BufReader<R>, buffer: &mut String) -> Result<(), String> {
    for _ in 0..count {
        read_line(reader, buffer).map_err(|_| "cannot skip lines")?;
    }

    Ok(())
}
