use crate::construction::heuristics::{create_end_activity, create_start_activity, RouteContext, RouteState};
use crate::helpers::models::problem::*;
use crate::models::common::{Duration, Location, Schedule, TimeWindow};
use crate::models::problem::{Actor, Fleet, Single};
use crate::models::solution::{Activity, Place, Route, Tour, TourActivity};
use std::sync::Arc;

pub const DEFAULT_ACTIVITY_SCHEDULE: Schedule = Schedule { departure: 0.0, arrival: 0.0 };

pub fn test_activity() -> Activity {
    test_activity_with_job(Arc::new(test_single()))
}

pub fn test_activity_with_location(location: Location) -> Activity {
    Activity {
        place: Place { location, duration: DEFAULT_JOB_DURATION, time: DEFAULT_ACTIVITY_TIME_WINDOW },
        schedule: Schedule::new(location as f64, location as f64 + DEFAULT_JOB_DURATION),
        job: Some(test_single_with_location(Some(location))),
    }
}

pub fn test_activity_with_location_and_duration(location: Location, duration: Duration) -> Activity {
    Activity {
        place: Place { location, duration, time: DEFAULT_ACTIVITY_TIME_WINDOW },
        schedule: Schedule::new(location as f64, location as f64 + DEFAULT_JOB_DURATION),
        job: Some(test_single_with_location(Some(location))),
    }
}

pub fn test_activity_with_location_and_tw(location: Location, tw: TimeWindow) -> Activity {
    Activity {
        place: Place { location, duration: DEFAULT_JOB_DURATION, time: tw },
        schedule: Schedule::new(location as f64, location as f64 + DEFAULT_JOB_DURATION),
        job: Some(test_single_with_location(Some(location))),
    }
}

pub fn test_activity_with_schedule(schedule: Schedule) -> Activity {
    Activity {
        place: Place {
            location: DEFAULT_JOB_LOCATION,
            duration: DEFAULT_JOB_DURATION,
            time: DEFAULT_ACTIVITY_TIME_WINDOW,
        },
        schedule,
        job: None,
    }
}

pub fn test_activity_with_job(job: Arc<Single>) -> Activity {
    Activity {
        place: Place {
            location: DEFAULT_JOB_LOCATION,
            duration: DEFAULT_JOB_DURATION,
            time: DEFAULT_ACTIVITY_TIME_WINDOW,
        },
        schedule: DEFAULT_ACTIVITY_SCHEDULE,
        job: Some(job),
    }
}

pub fn test_activity_without_job() -> Activity {
    Activity {
        place: Place {
            location: DEFAULT_JOB_LOCATION,
            duration: DEFAULT_JOB_DURATION,
            time: DEFAULT_ACTIVITY_TIME_WINDOW,
        },
        schedule: DEFAULT_ACTIVITY_SCHEDULE,
        job: None,
    }
}

pub fn create_route_with_start_end_activities(
    fleet: &Fleet,
    vehicle: &str,
    start: TourActivity,
    end: TourActivity,
    activities: Vec<TourActivity>,
) -> Route {
    let mut tour = Tour::default();
    tour.set_start(start);
    tour.set_end(end);

    create_route(get_test_actor_from_fleet(fleet, vehicle), tour, activities)
}

pub fn create_route_with_activities(fleet: &Fleet, vehicle: &str, activities: Vec<TourActivity>) -> Route {
    let actor = get_test_actor_from_fleet(fleet, vehicle);
    let mut tour = Tour::default();
    tour.set_start(create_start_activity(&actor));
    create_end_activity(&actor).map(|end| tour.set_end(end));

    create_route(actor, tour, activities)
}

pub fn create_route_context_with_activities(
    fleet: &Fleet,
    vehicle: &str,
    activities: Vec<TourActivity>,
) -> RouteContext {
    let route = create_route_with_activities(fleet, vehicle, activities);

    RouteContext { route: Arc::new(route), state: Arc::new(RouteState::default()) }
}

fn create_route(actor: Arc<Actor>, mut tour: Tour, activities: Vec<TourActivity>) -> Route {
    activities.into_iter().enumerate().for_each(|(index, a)| {
        tour.insert_at(a, index + 1);
    });

    Route { actor, tour }
}

pub struct ActivityBuilder {
    activity: Activity,
}

impl ActivityBuilder {
    pub fn new() -> Self {
        Self { activity: test_activity() }
    }

    pub fn place(&mut self, place: Place) -> &mut Self {
        self.activity.place = place;
        self
    }

    pub fn schedule(&mut self, schedule: Schedule) -> &mut Self {
        self.activity.schedule = schedule;
        self
    }

    pub fn job(&mut self, job: Option<Arc<Single>>) -> &mut Self {
        self.activity.job = job;
        self
    }

    pub fn build(&mut self) -> Activity {
        std::mem::replace(&mut self.activity, test_activity())
    }
}
