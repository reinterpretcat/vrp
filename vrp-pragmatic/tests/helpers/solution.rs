//! Provides helper logic for solution domain, sometimes assuming some defaults used from problem domain helpers.

use crate::format::solution::*;
use crate::format::{CustomLocationType, Location};
use crate::format_time;
use crate::helpers::ToLocation;
use std::cmp::Ordering::Equal;
use std::collections::HashMap;
use std::io::{BufReader, BufWriter};
use std::sync::Arc;
use vrp_core::models::common::{Cost, Timestamp};
use vrp_core::models::Problem as CoreProblem;
use vrp_core::models::Solution as CoreSolution;
use vrp_core::prelude::{Float, GenericError};
use vrp_core::utils::{DefaultRandom, Random};

/// Provides way to build a stop with one or multiple activities.
pub struct StopBuilder {
    stop: Stop,
}

impl StopBuilder {
    pub fn new_transit() -> Self {
        Self {
            stop: Stop::Transit(TransitStop {
                time: Schedule { arrival: format_time(0.), departure: format_time(0.) },
                load: vec![],
                activities: vec![],
            }),
        }
    }

    pub fn distance(mut self, distance: i64) -> Self {
        let mut stop = self.stop.to_point();
        stop.distance = distance;
        self.stop = Stop::Point(stop);

        self
    }

    pub fn coordinate(mut self, coordinate: (f64, f64)) -> Self {
        let mut stop = self.stop.to_point();
        stop.location = coordinate.to_loc();
        self.stop = Stop::Point(stop);

        self
    }

    pub fn reference(mut self, index: usize) -> Self {
        let mut stop = self.stop.to_point();
        stop.location = Location::Reference { index };
        self.stop = Stop::Point(stop);

        self
    }

    pub fn custom_unknown(mut self) -> Self {
        let mut stop = self.stop.to_point();
        stop.location = Location::Custom { r#type: CustomLocationType::Unknown };
        self.stop = Stop::Point(stop);

        self
    }

    pub fn load(mut self, load: Vec<i32>) -> Self {
        *self.stop.load_mut() = load;

        self
    }

    pub fn schedule_stamp(mut self, arrival: Timestamp, departure: Timestamp) -> Self {
        *self.stop.schedule_mut() = Schedule { arrival: format_time(arrival), departure: format_time(departure) };

        self
    }

    pub fn activity(mut self, activity: Activity) -> Self {
        self.stop.activities_mut().push(activity);

        self
    }

    pub fn activities(mut self, activities: Vec<Activity>) -> Self {
        self.stop.activities_mut().extend(activities);

        self
    }

    /// Builds a stop with one or more activities defined by the user.
    pub fn build(self) -> Stop {
        if self.stop.load().is_empty() {
            panic!("no load is set");
        }

        if self.stop.activities().is_empty() {
            panic!("no activities are set");
        }

        self.stop
    }

    /// Builds departure stop with single predefined departure activity.
    pub fn build_departure(mut self) -> Stop {
        if !self.stop.activities().is_empty() {
            panic!("non empty departure list of activities, use alternatives");
        }

        self = self.distance(0);

        self.stop
            .activities_mut()
            .push(ActivityBuilder::default().job_id("departure").activity_type("departure").build());

        self.stop
    }

    /// Builds arrival stop with single predefined arrival activity.
    pub fn build_arrival(mut self) -> Stop {
        if !self.stop.activities().is_empty() {
            panic!("non empty arrival list of activities, use alternatives");
        }

        self.stop.activities_mut().push(ActivityBuilder::default().job_id("arrival").activity_type("arrival").build());

        self.stop
    }

    /// Builds a stop with single predefined activity with given type, job id and tag.
    pub fn build_single_tag(mut self, job_id: &str, activity_type: &str, tag: &str) -> Stop {
        if !self.stop.activities().is_empty() {
            panic!("non empty single list of activities, use alternatives");
        }

        self = self.activity(ActivityBuilder::default().activity_type(activity_type).job_id(job_id).tag(tag).build());

        self.stop
    }

    /// Builds a stop with single predefined activity with given type, job id and time.
    pub fn build_single_time(mut self, job_id: &str, activity_type: &str, time: (Timestamp, Timestamp)) -> Stop {
        if !self.stop.activities().is_empty() {
            panic!("non empty single list of activities, use alternatives");
        }

        self = self.activity(
            ActivityBuilder::default().activity_type(activity_type).job_id(job_id).time_stamp(time.0, time.1).build(),
        );

        self.stop
    }

    /// Builds a stop with single predefined activity with given type and job id.
    pub fn build_single(mut self, job_id: &str, activity_type: &str) -> Stop {
        if !self.stop.activities().is_empty() {
            panic!("non empty single list of activities, use alternatives");
        }

        self = self.activity(ActivityBuilder::default().activity_type(activity_type).job_id(job_id).build());

        self.stop
    }
}

impl Default for StopBuilder {
    fn default() -> Self {
        Self {
            stop: Stop::Point(PointStop {
                location: Location::Coordinate { lat: 0., lng: 0. },
                time: Schedule { arrival: format_time(0.), departure: format_time(0.) },
                distance: 0,
                load: vec![],
                parking: None,
                activities: vec![],
            }),
        }
    }
}

pub struct ActivityBuilder {
    activity: Activity,
}

impl ActivityBuilder {
    pub fn delivery() -> Self {
        let mut builder = Self::default();
        builder.activity.activity_type = "delivery".to_string();

        builder
    }

    pub fn pickup() -> Self {
        let mut builder = Self::default();
        builder.activity.activity_type = "pickup".to_string();

        builder
    }

    pub fn break_type() -> Self {
        let mut builder = Self::default();
        builder.activity.activity_type = "break".to_string();
        builder.activity.job_id = "break".to_string();

        builder
    }

    pub fn job_id(mut self, job_id: &str) -> Self {
        self.activity.job_id = job_id.to_string();

        self
    }

    pub fn activity_type(mut self, activity_type: &str) -> Self {
        self.activity.activity_type = activity_type.to_string();

        self
    }

    pub fn coordinate(mut self, coordinate: (f64, f64)) -> Self {
        self.activity.location = Some(coordinate.to_loc());

        self
    }

    pub fn time_stamp(mut self, start: Timestamp, end: Timestamp) -> Self {
        self.activity.time = Some(Interval { start: format_time(start), end: format_time(end) });

        self
    }

    pub fn tag(mut self, tag: &str) -> Self {
        self.activity.job_tag = Some(tag.to_string());

        self
    }

    pub fn commute(mut self, commute: Commute) -> Self {
        self.activity.commute = Some(commute);

        self
    }

    pub fn build(self) -> Activity {
        if self.activity.activity_type.is_empty() {
            panic!("missing activity type");
        }

        if self.activity.job_id.is_empty() {
            panic!("missing activity job id")
        }

        self.activity
    }
}

impl Default for ActivityBuilder {
    fn default() -> Self {
        Self {
            activity: Activity {
                job_id: "".to_string(),
                activity_type: "".to_string(),
                location: None,
                time: None,
                job_tag: None,
                commute: None,
            },
        }
    }
}

pub struct StatisticBuilder {
    fixed: Cost,
    costs: (Cost, Cost),
    statistic: Statistic,
}

impl StatisticBuilder {
    pub fn driving(mut self, driving: i64) -> Self {
        self.statistic.times.driving = driving;

        self
    }

    pub fn serving(mut self, serving: i64) -> Self {
        self.statistic.times.serving = serving;

        self
    }

    pub fn waiting(mut self, waiting: i64) -> Self {
        self.statistic.times.waiting = waiting;

        self
    }

    pub fn break_time(mut self, break_time: i64) -> Self {
        self.statistic.times.break_time = break_time;

        self
    }

    pub fn build(self) -> Statistic {
        let mut statistic = self.statistic;
        let (per_distance, per_time) = self.costs;
        let times = statistic.times.clone();

        statistic.duration =
            times.driving + times.serving + times.waiting + times.break_time + times.parking + times.commuting;
        statistic.distance = statistic.times.driving;
        statistic.cost =
            self.fixed + statistic.distance as Float * per_distance + statistic.duration as Float * per_time;

        statistic
    }
}

impl Default for StatisticBuilder {
    fn default() -> Self {
        Self { fixed: 10.0, costs: (1., 1.), statistic: Default::default() }
    }
}

pub struct TourBuilder {
    tour: Tour,
}

impl TourBuilder {
    pub fn type_id(mut self, id: &str) -> Self {
        self.tour.type_id = id.to_string();

        self
    }

    pub fn vehicle_id(mut self, id: &str) -> Self {
        self.tour.vehicle_id = id.to_string();

        self
    }

    pub fn shift_index(mut self, idx: usize) -> Self {
        self.tour.shift_index = idx;

        self
    }

    pub fn stops(mut self, stops: Vec<Stop>) -> Self {
        self.tour.stops = stops;

        self
    }

    pub fn statistic(mut self, statistic: Statistic) -> Self {
        self.tour.statistic = statistic;

        self
    }

    pub fn build(self) -> Tour {
        if self.tour.stops.is_empty() {
            panic!("no stops in the tour");
        }

        self.tour
    }
}

impl Default for TourBuilder {
    fn default() -> Self {
        Self {
            tour: Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![],
                statistic: Default::default(),
            },
        }
    }
}

pub struct SolutionBuilder {
    solution: Solution,
}

impl SolutionBuilder {
    pub fn tour(mut self, tour: Tour) -> Self {
        self.solution.tours.push(tour);

        self
    }

    pub fn unassigned(mut self, unassigned: Option<Vec<UnassignedJob>>) -> Self {
        self.solution.unassigned = unassigned;

        self
    }

    pub fn violations(mut self, violations: Option<Vec<Violation>>) -> Self {
        self.solution.violations = violations;

        self
    }

    pub fn build(mut self) -> Solution {
        self.solution.statistic =
            self.solution.tours.iter().fold(Statistic::default(), |acc, tour| acc + tour.statistic.clone());

        self.solution
    }
}

impl Default for SolutionBuilder {
    fn default() -> Self {
        Self {
            solution: Solution {
                statistic: Default::default(),
                tours: vec![],
                unassigned: None,
                violations: None,
                extras: None,
            },
        }
    }
}

pub fn assert_vehicle_agnostic(result: Solution, expected: Solution) {
    let mut result = result;

    let tour_map = expected.tours.iter().fold(HashMap::new(), |mut acc, tour| {
        acc.insert(tour.stops.get(1).unwrap().activities().first().unwrap().job_id.clone(), tour.vehicle_id.clone());

        acc
    });

    result.tours.iter_mut().for_each(|tour| {
        let job_id = tour.stops.get(1).unwrap().activities().first().unwrap().job_id.clone();
        if let Some(vehicle_id) = tour_map.get(&job_id) {
            tour.vehicle_id = vehicle_id.to_string();
        }
    });

    result.tours.sort_by(|a, b| {
        let ordering = a.vehicle_id.cmp(&b.vehicle_id);

        if ordering == Equal {
            a.shift_index.cmp(&b.shift_index)
        } else {
            ordering
        }
    });

    assert_eq!(result, expected);
}

pub fn get_ids_from_tour(tour: &Tour) -> Vec<Vec<String>> {
    tour.stops.iter().map(|stop| stop.activities().iter().map(|a| a.job_id.clone()).collect()).collect()
}

pub fn get_ids_from_tour_sorted(tour: &Tour) -> Vec<Vec<String>> {
    let mut ids = get_ids_from_tour(tour);
    ids.sort();

    ids
}

pub fn create_random() -> Arc<dyn Random> {
    Arc::new(DefaultRandom::default())
}

pub fn to_core_solution(
    solution: &Solution,
    core_problem: Arc<CoreProblem>,
    random: Arc<dyn Random>,
) -> Result<CoreSolution, GenericError> {
    let mut writer = BufWriter::new(Vec::new());
    serialize_solution(solution, &mut writer).expect("cannot serialize test solution");
    let bytes = writer.into_inner().expect("cannot get bytes from writer");

    read_init_solution(BufReader::new(bytes.as_slice()), core_problem, random)
}
