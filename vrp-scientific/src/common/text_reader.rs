use std::io::prelude::*;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::construction::features::*;
use vrp_core::models::common::*;
use vrp_core::models::problem::*;
use vrp_core::models::*;
use vrp_core::prelude::GenericError;

pub(crate) trait TextReader {
    fn read_problem(&mut self, is_rounded: bool) -> Result<Problem, GenericError> {
        let (jobs, fleet) = self.read_definitions()?;
        let transport = self.create_transport(is_rounded)?;
        let activity = Arc::new(SimpleActivityCost::default());
        let jobs = Jobs::new(&fleet, jobs, &transport);
        let goal = self.create_goal_context(activity.clone(), transport.clone()).expect("cannot create goal context");

        Ok(Problem {
            fleet: Arc::new(fleet),
            jobs: Arc::new(jobs),
            locks: vec![],
            goal: Arc::new(goal),
            activity,
            transport,
            extras: Arc::new(self.create_extras()),
        })
    }

    fn create_goal_context(
        &self,
        activity: Arc<SimpleActivityCost>,
        transport: Arc<dyn TransportCost + Send + Sync>,
    ) -> Result<GoalContext, GenericError>;

    fn read_definitions(&mut self) -> Result<(Vec<Job>, Fleet), GenericError>;

    fn create_transport(&self, is_rounded: bool) -> Result<Arc<dyn TransportCost + Send + Sync>, GenericError>;

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
    let mut dimens = Dimensions::default();
    dimens.set_id([prefix.to_string(), id.to_string()].concat().as_str());
    dimens
}

pub(crate) fn create_goal_context_prefer_min_tours(
    activity: Arc<SimpleActivityCost>,
    transport: Arc<dyn TransportCost + Send + Sync>,
) -> Result<GoalContext, GenericError> {
    let features = get_essential_features(activity, transport)?;

    let goal = Goal::with_alternatives(
        vec![vec!["min_unassigned".to_string()], vec!["min_tours".to_string()], vec!["min_distance".to_string()]],
        vec![vec!["min_tours".to_string()], vec!["min_distance".to_string()]],
        (
            vec![(
                vec![vec!["min_unassigned".to_string()], vec!["min_distance".to_string()]],
                vec![vec!["min_distance".to_string()]],
            )],
            0.1,
        ),
    );

    GoalContext::new(features.as_slice(), goal)
}

pub(crate) fn create_goal_context_distance_only(
    activity: Arc<SimpleActivityCost>,
    transport: Arc<dyn TransportCost + Send + Sync>,
) -> Result<GoalContext, GenericError> {
    let features = get_essential_features(activity, transport)?;

    let goal = Goal::with_alternatives(
        vec![vec!["min_unassigned".to_string()], vec!["min_distance".to_string()]],
        vec![vec!["min_distance".to_string()]],
        (
            vec![(
                vec![
                    vec!["min_unassigned".to_string()],
                    vec!["min_tours".to_string()],
                    vec!["min_distance".to_string()],
                ],
                vec![vec!["min_tours".to_string()], vec!["min_distance".to_string()]],
            )],
            0.1,
        ),
    );

    GoalContext::new(features.as_slice(), goal)
}

fn get_essential_features(
    activity: Arc<SimpleActivityCost>,
    transport: Arc<dyn TransportCost + Send + Sync>,
) -> Result<Vec<Feature>, GenericError> {
    Ok(vec![
        create_minimize_unassigned_jobs_feature("min_unassigned", Arc::new(|_, _| 1.))?,
        create_minimize_tours_feature("min_tours")?,
        create_minimize_distance_feature("min_distance", transport, activity, 1)?,
        create_capacity_limit_feature::<SingleDimLoad>("capacity", 2)?,
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
