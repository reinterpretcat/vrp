//! Import from an another json format logic.

extern crate serde_json;

use serde::{Deserialize, Serialize};
use std::io::{BufReader, Read};
use vrp_pragmatic::format::problem::*;
use vrp_pragmatic::format::{FormatError, Location};

mod hre {
    use super::*;

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Location {
        /// Latitude.
        pub lat: f64,
        /// Longitude.
        pub lng: f64,
    }

    // region Plan

    /// Relation type.
    #[derive(Clone, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub enum RelationType {
        /// Tour relation locks jobs to specific vehicle in any order.
        Tour,
        /// Flexible relation locks jobs in specific order allowing insertion of other jobs in between.
        Flexible,
        /// Sequence relation locks jobs in strict order, no insertions in between are allowed.
        Sequence,
    }

    /// Relation is the way to lock specific jobs to specific vehicles.
    #[derive(Clone, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Relation {
        /// Relation type.
        #[serde(rename(deserialize = "type"))]
        pub type_field: RelationType,
        /// List of job ids.
        pub jobs: Vec<String>,
        /// Vehicle id.
        pub vehicle_id: String,
        /// Vehicle shift index.
        pub shift_index: Option<usize>,
    }

    /// Defines specific job place.
    #[derive(Clone, Deserialize, Debug)]
    pub struct JobPlace {
        /// A list of job time windows with time specified in RFC3339 format.
        pub times: Option<Vec<Vec<String>>>,
        /// Job location.
        pub location: Location,
        /// Job duration (service time).
        pub duration: f64,
        /// An tag which will be propagated back within corresponding activity in solution.
        pub tag: Option<String>,
    }

    /// Specifies pickup and delivery places of the job.
    /// At least one place should be specified. If only delivery specified, then vehicle is loaded with
    /// job's demand at the start location. If only pickup specified, then loaded good is delivered to
    /// the last location on the route. When both, pickup and delivery, are specified, then it is classical
    /// pickup and delivery job.
    #[derive(Clone, Deserialize, Debug)]
    pub struct JobPlaces {
        /// Pickup place.
        pub pickup: Option<JobPlace>,
        /// Delivery place.
        pub delivery: Option<JobPlace>,
    }

    /// Specifies single job.
    #[derive(Clone, Deserialize, Debug)]
    pub struct Job {
        /// Job id.
        pub id: String,
        /// Job places.
        pub places: JobPlaces,
        /// Job demand.
        pub demand: Vec<i32>,
        /// Job priority, bigger value - less important.
        pub priority: Option<i32>,
        /// Job skills.
        pub skills: Option<Vec<String>>,
    }

    /// Specifies a place for sub job.
    #[derive(Clone, Deserialize, Debug)]
    pub struct MultiJobPlace {
        /// A list of sub job time windows with time specified in RFC3339 format.
        pub times: Option<Vec<Vec<String>>>,
        /// Sub job location.
        pub location: Location,
        /// Sub job duration (service time).
        pub duration: f64,
        /// Sub job demand.
        pub demand: Vec<i32>,
        /// An tag which will be propagated back within corresponding activity in solution.
        pub tag: Option<String>,
    }

    /// Specifies pickups and deliveries places of multi job.
    /// All of them should be completed or none of them. All pickups must be completed before any of deliveries.
    #[derive(Clone, Deserialize, Debug)]
    pub struct MultiJobPlaces {
        /// A list of pickups.
        pub pickups: Vec<MultiJobPlace>,
        /// A list of deliveries.
        pub deliveries: Vec<MultiJobPlace>,
    }

    /// Specifies multi job which has multiple child jobs.
    #[derive(Clone, Deserialize, Debug)]
    pub struct MultiJob {
        /// Multi job id.
        pub id: String,
        /// Multi job places.
        pub places: MultiJobPlaces,
        /// Job priority, bigger value - less important.
        pub priority: Option<i32>,
        /// Multi job skills.
        pub skills: Option<Vec<String>>,
    }

    /// Job variant type.
    #[derive(Clone, Deserialize, Debug)]
    #[serde(untagged)]
    pub enum JobVariant {
        /// Single job.
        Single(Job),
        /// Multi job.
        Multi(MultiJob),
    }

    /// A plan specifies work which has to be done.
    #[derive(Clone, Deserialize, Debug)]
    pub struct Plan {
        /// List of jobs.
        pub jobs: Vec<JobVariant>,
        /// List of relations between jobs and vehicles.
        pub relations: Option<Vec<Relation>>,
    }

    // endregion

    // region Fleet

    /// Specifies vehicle costs.
    #[derive(Clone, Deserialize, Debug)]
    pub struct VehicleCosts {
        /// Fixed is cost of vehicle usage per tour.
        pub fixed: Option<f64>,
        /// Cost per distance unit.
        pub distance: f64,
        /// Cost per time unit.
        pub time: f64,
    }

    /// Specifies vehicle place.
    #[derive(Clone, Deserialize, Debug)]
    pub struct VehiclePlace {
        /// Vehicle start or end time.
        pub time: String,
        /// Vehicle location.
        pub location: Location,
    }

    /// Specifies vehicle shift.
    #[derive(Clone, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct VehicleShift {
        /// Vehicle start place.
        pub start: VehiclePlace,
        /// Vehicle end place.
        pub end: Option<VehiclePlace>,
        /// Vehicle breaks.
        pub breaks: Option<Vec<VehicleBreak>>,
        /// Vehicle reloads which allows vehicle to return back to the depot (or any other place) in
        /// order to unload/load goods during single tour.
        pub reloads: Option<Vec<VehicleReload>>,
    }

    /// Vehicle reload.
    pub type VehicleReload = JobPlace;

    /// Vehicle limits.
    #[derive(Clone, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct VehicleLimits {
        /// Max traveling distance per shift/tour.
        pub max_distance: Option<f64>,
        /// Max time per shift/tour.
        pub shift_time: Option<f64>,
    }

    /// Vehicle break.
    #[derive(Clone, Deserialize, Debug)]
    pub struct VehicleBreak {
        /// Break time.
        pub times: Vec<Vec<String>>,
        /// Break duration.
        pub duration: f64,
        /// Break location.
        pub location: Option<Location>,
    }

    /// Specifies a vehicle type.
    #[derive(Clone, Deserialize, Debug)]
    pub struct VehicleType {
        /// Vehicle type id.
        pub id: String,
        /// Vehicle profile name.
        pub profile: String,
        /// Vehicle costs.
        pub costs: VehicleCosts,
        /// Vehicle shifts.
        pub shifts: Vec<VehicleShift>,
        /// Vehicle capacity.
        pub capacity: Vec<i32>,
        /// Vehicle amount.
        pub amount: i32,
        /// Vehicle skills.
        pub skills: Option<Vec<String>>,
        /// Vehicle limits.
        pub limits: Option<VehicleLimits>,
    }

    /// Specifies routing profile.
    #[derive(Clone, Deserialize, Debug)]
    pub struct Profile {
        /// Profile name.
        pub name: String,
        /// Profile type.
        #[serde(rename(deserialize = "type"))]
        pub profile_type: String,
    }

    /// Specifies fleet.
    #[derive(Clone, Deserialize, Debug)]
    pub struct Fleet {
        /// Vehicle types.
        pub types: Vec<VehicleType>,
        /// Routing profiles.
        pub profiles: Vec<Profile>,
    }

    // endregion

    // region Configuration

    /// Specifies extra configuration.
    #[derive(Clone, Deserialize, Debug)]
    pub struct Config {
        /// Features config.
        pub features: Option<Features>,
    }

    /// Specifies features config.
    #[derive(Clone, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Features {
        /// Even distribution of the jobs across tours. By default, is off.
        pub even_distribution: Option<EvenDistribution>,
        /// Tweaks priority weight. Default value is 100.
        pub priority: Option<Priority>,
    }

    /// Configuration to tweak even distribution of the jobs across tours.
    #[derive(Clone, Deserialize, Debug)]
    pub struct EvenDistribution {
        /// Enable or disable.
        pub enabled: bool,
        /// A fraction of this cost is applied when jobs are assigned to the tour.
        pub extra_cost: Option<f64>,
    }

    /// Configuration to tweak even distribution of the jobs across tours.
    #[derive(Clone, Deserialize, Debug)]
    pub struct Priority {
        /// A cost for formula: `extra_cost = (priority - 1) * weight_cost`.
        pub weight_cost: f64,
    }

    // endregion

    // region Common

    /// A VRP problem definition.
    #[derive(Clone, Deserialize, Debug)]
    pub struct Problem {
        /// Problem plan: customers to serve.
        pub plan: Plan,
        /// Problem resources: vehicles to be used, routing info.
        pub fleet: Fleet,
        /// Extra configuration.
        pub config: Option<Config>,
    }

    // endregion
}

fn to_loc(loc: &hre::Location) -> Location {
    Location { lat: loc.lat, lng: loc.lng }
}

pub fn read_hre_problem<R: Read>(reader: BufReader<R>) -> Result<Problem, FormatError> {
    let job_place_mapper = |job: &hre::Job, place: &hre::JobPlace| JobTask {
        places: vec![JobPlace {
            location: to_loc(&place.location),
            duration: place.duration,
            times: place.times.clone(),
        }],
        demand: Some(job.demand.clone()),
        tag: place.tag.clone(),
    };

    let multi_job_place_mapper = |places: &Vec<hre::MultiJobPlace>| {
        if places.is_empty() {
            None
        } else {
            Some(
                places
                    .iter()
                    .map(|place| JobTask {
                        places: vec![JobPlace {
                            location: to_loc(&place.location),
                            duration: place.duration,
                            times: place.times.clone(),
                        }],
                        demand: Some(place.demand.clone()),
                        tag: place.tag.clone(),
                    })
                    .collect(),
            )
        }
    };

    let hre_problem: hre::Problem = serde_json::from_reader(reader)
        .map_err(|err| FormatError::new("E0000".to_string(), err.to_string(), "Check input json".to_string()))?;

    Ok(Problem {
        plan: Plan {
            jobs: hre_problem
                .plan
                .jobs
                .iter()
                .map(|job| match job {
                    hre::JobVariant::Single(job) => Job {
                        id: job.id.clone(),
                        pickups: job.places.pickup.as_ref().map(|place| vec![job_place_mapper(job, place)]),
                        deliveries: job.places.delivery.as_ref().map(|place| vec![job_place_mapper(job, place)]),
                        replacements: None,
                        services: None,
                        priority: job.priority.as_ref().map(|p| *p),
                        skills: job.skills.clone(),
                    },
                    hre::JobVariant::Multi(job) => Job {
                        id: job.id.clone(),
                        pickups: multi_job_place_mapper(&job.places.pickups),
                        deliveries: multi_job_place_mapper(&job.places.deliveries),
                        replacements: None,
                        services: None,
                        priority: job.priority.as_ref().map(|p| *p),
                        skills: job.skills.clone(),
                    },
                })
                .collect(),
            relations: hre_problem.plan.relations.map(|relations| {
                relations
                    .iter()
                    .map(|r| Relation {
                        type_field: match r.type_field {
                            hre::RelationType::Sequence => RelationType::Strict,
                            hre::RelationType::Flexible => RelationType::Sequence,
                            hre::RelationType::Tour => RelationType::Any,
                        },
                        jobs: r.jobs.clone(),
                        vehicle_id: r.vehicle_id.clone(),
                        shift_index: r.shift_index.clone(),
                    })
                    .collect()
            }),
        },
        fleet: Fleet {
            vehicles: hre_problem
                .fleet
                .types
                .iter()
                .map(|v| VehicleType {
                    type_id: v.id.clone(),
                    vehicle_ids: (1..=v.amount).map(|seq| format!("{}_{}", v.id, seq)).collect(),
                    profile: v.profile.clone(),
                    costs: VehicleCosts {
                        fixed: v.costs.fixed.clone(),
                        distance: v.costs.distance,
                        time: v.costs.time,
                    },
                    shifts: v
                        .shifts
                        .iter()
                        .map(|shift| VehicleShift {
                            start: VehiclePlace {
                                time: shift.start.time.clone(),
                                location: to_loc(&shift.start.location),
                            },
                            end: shift
                                .end
                                .as_ref()
                                .map(|end| VehiclePlace { time: end.time.clone(), location: to_loc(&end.location) }),
                            breaks: shift.breaks.as_ref().map(|breaks| {
                                breaks
                                    .iter()
                                    .map(|b| VehicleBreak {
                                        time: VehicleBreakTime::TimeWindow(b.times.first().unwrap().clone()),
                                        duration: b.duration,
                                        locations: b.location.as_ref().map(|l| vec![to_loc(l)]),
                                    })
                                    .collect()
                            }),
                            reloads: shift.reloads.as_ref().map(|reloads| {
                                reloads
                                    .iter()
                                    .map(|r| VehicleReload {
                                        location: to_loc(&r.location),
                                        duration: r.duration.clone(),
                                        times: r.times.clone(),
                                        tag: r.tag.clone(),
                                    })
                                    .collect()
                            }),
                        })
                        .collect(),
                    capacity: v.capacity.clone(),
                    skills: v.skills.clone(),
                    limits: v.limits.as_ref().map(|l| VehicleLimits {
                        max_distance: l.max_distance.clone(),
                        shift_time: l.shift_time.clone(),
                        allowed_areas: None,
                    }),
                })
                .collect(),
            profiles: hre_problem
                .fleet
                .profiles
                .iter()
                .map(|p| Profile { name: p.name.clone(), profile_type: p.profile_type.clone() })
                .collect(),
        },
        objectives: None,
        config: None,
    })
}
