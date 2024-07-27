use std::io::prelude::*;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::construction::features::*;
use vrp_core::models::common::*;
use vrp_core::models::problem::*;
use vrp_core::models::*;
use vrp_core::prelude::GenericError;
use vrp_core::utils::GenericResult;

pub(crate) trait TextReader {
    fn read_problem(&mut self, is_rounded: bool) -> Result<Problem, GenericError> {
        let (jobs, fleet) = self.read_definitions()?;
        let transport = self.create_transport(is_rounded)?;
        let activity = Arc::new(SimpleActivityCost::default());
        let jobs = Jobs::new(&fleet, jobs, transport.as_ref());
        let extras = self.create_extras();
        let goal = self.create_goal_context(activity.clone(), transport.clone()).expect("cannot create goal context");

        Ok(Problem {
            fleet: Arc::new(fleet),
            jobs: Arc::new(jobs),
            locks: vec![],
            goal: Arc::new(goal),
            activity,
            transport,
            extras: Arc::new(extras),
        })
    }

    fn create_goal_context(
        &self,
        activity: Arc<SimpleActivityCost>,
        transport: Arc<dyn TransportCost>,
    ) -> Result<GoalContext, GenericError>;

    fn read_definitions(&mut self) -> Result<(Vec<Job>, Fleet), GenericError>;

    fn create_transport(&self, is_rounded: bool) -> Result<Arc<dyn TransportCost>, GenericError>;

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
            dimens: Default::default(),
            details: Default::default(),
        })],
        (0..number)
            .map(|i| {
                let mut dimens = create_dimens_with_id("v", &i.to_string(), |id, dimens| {
                    dimens.set_vehicle_id(id.to_string());
                });
                dimens.set_vehicle_capacity(SingleDimLoad::new(capacity as i32));
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
        |_| |_| 0,
    )
}

pub(crate) fn create_dimens_with_id(
    prefix: &str,
    id: &str,
    id_setter_fn: impl Fn(&str, &mut Dimensions),
) -> Dimensions {
    let mut dimens = Dimensions::default();
    id_setter_fn([prefix.to_string(), id.to_string()].concat().as_str(), &mut dimens);
    dimens
}

pub(crate) fn create_goal_context_prefer_min_tours(
    activity: Arc<SimpleActivityCost>,
    transport: Arc<dyn TransportCost>,
    is_time_constrained: bool,
) -> GenericResult<GoalContext> {
    let features = get_essential_features(activity, transport, is_time_constrained)?;

    GoalContextBuilder::with_features(&features)?
        .set_main_goal(Goal::subset_of(&features, &["min_unassigned", "min_tours", "min_distance"])?)
        .add_alternative_goal(Goal::subset_of(&features, &["min_unassigned", "min_distance"])?, 0.1)
        .build()
}

pub(crate) fn create_goal_context_distance_only(
    activity: Arc<SimpleActivityCost>,
    transport: Arc<dyn TransportCost>,
    is_time_constrained: bool,
) -> Result<GoalContext, GenericError> {
    let features = get_essential_features(activity, transport, is_time_constrained)?;

    GoalContextBuilder::with_features(&features)?
        .set_main_goal(Goal::subset_of(&features, &["min_unassigned", "min_distance"])?)
        .add_alternative_goal(Goal::subset_of(&features, &["min_unassigned", "min_tours", "min_distance"])?, 0.1)
        .build()
}

fn get_essential_features(
    activity: Arc<SimpleActivityCost>,
    transport: Arc<dyn TransportCost>,
    is_time_constrained: bool,
) -> Result<Vec<Feature>, GenericError> {
    Ok(vec![
        MinimizeUnassignedBuilder::new("min_unassigned").build()?,
        create_minimize_tours_feature("min_tours")?,
        TransportFeatureBuilder::new("min_distance")
            .set_time_constrained(is_time_constrained)
            .set_transport_cost(transport)
            .set_activity_cost(activity)
            .build_minimize_distance()?,
        CapacityFeatureBuilder::<SingleDimLoad>::new("capacity").build()?,
    ])
}

pub(crate) fn read_line<R: Read>(reader: &mut BufReader<R>, buffer: &mut String) -> Result<usize, GenericError> {
    buffer.clear();
    reader.read_line(buffer).map_err(|err| err.to_string().into())
}

pub(crate) fn skip_lines<R: Read>(
    count: usize,
    reader: &mut BufReader<R>,
    buffer: &mut String,
) -> Result<(), GenericError> {
    for _ in 0..count {
        read_line(reader, buffer).map_err(|_| "cannot skip lines")?;
    }

    Ok(())
}
