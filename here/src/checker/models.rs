use crate::json::problem::*;
use crate::json::solution::*;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct VehicleMeta {
    pub vehicle_id: String,
    pub vehicle_type: Arc<VehicleType>,
}

pub enum ActivityPlace {
    Single(JobPlace),
    Multi(MultiJobPlace),
    Break(VehicleBreak),
    Vehicle(VehiclePlace),
}

pub struct ActivityInfo {
    pub activity: Activity,
    pub job_id: Option<String>,
    pub job: Option<Arc<JobVariant>>,
    pub vehicle_meta: VehicleMeta,
    pub schedule: (f64, f64),
}

pub struct TourInfo {
    pub vehicle_meta: VehicleMeta,
    pub activities: Vec<ActivityInfo>,
    pub schedule: (f64, f64),
}

pub struct RelationInfo {
    pub relation: Relation,
    pub vehicle_type: Arc<VehicleType>,
    pub jobs: Vec<Option<Arc<JobVariant>>>,
}

pub struct UnassignedInfo {
    pub unassigned: UnassignedJob,
    pub job: Arc<JobVariant>,
}

pub struct SolutionInfo {
    pub jobs: HashMap<String, Arc<JobVariant>>,
    pub tours: Vec<TourInfo>,
    pub relations: Vec<RelationInfo>,
    pub unassigned: Vec<UnassignedInfo>,
}

impl ActivityInfo {
    pub fn get_place(&self) -> Result<(ActivityPlace, usize), String> {
        if let Some(job) = self.job.as_ref() {
            match job.as_ref() {
                JobVariant::Single(job) => match self.activity.activity_type.as_str() {
                    "pickup" => {
                        let place = job
                            .places
                            .pickup
                            .clone()
                            .ok_or(format!("Pickup activity for job without pickup place: {}", job.id))?;
                        Ok((ActivityPlace::Single(place), 0))
                    }
                    "delivery" => {
                        let place = job
                            .places
                            .delivery
                            .clone()
                            .ok_or(format!("Delivery activity for job without delivery place: {}", job.id))?;
                        Ok((ActivityPlace::Single(place), 0))
                    }
                    _ => Err(format!("Invalid job activity type: '{}' for {}", self.activity.activity_type, job.id)),
                },
                JobVariant::Multi(job) => {
                    let tag = self
                        .activity
                        .job_tag
                        .as_ref()
                        .ok_or("UNSUPPORTED: solution checker requires multi job to have tags!")?;
                    match self.activity.activity_type.as_str() {
                        "pickup" => {
                            //let places: Vec<_> = job.places.pickups.iter().filter(|p| p.tag == tag).collect();

                            //                            job.places.pickups.iter().try_fold(Err("Cannot find pickup"), |acc, pickup| {
                            //
                            //                            })

                            unimplemented!()
                        }
                        "delivery" => unimplemented!(),
                        _ => {
                            Err(format!("Invalid job activity type: '{}' for {}", self.activity.activity_type, job.id))
                        }
                    }
                }
            }
        } else {
            let vehicle_type = &self.vehicle_meta.vehicle_type;
            let vehicle_id = &self.vehicle_meta.vehicle_id;
            match self.activity.activity_type.as_str() {
                "departure" => Ok((ActivityPlace::Vehicle(vehicle_type.places.start.clone()), 0)),
                "arrival" => {
                    let end_place = vehicle_type
                        .places
                        .end
                        .clone()
                        .ok_or(format!("Arrival activity for vehicle without end: {}", vehicle_id))?;
                    Ok((ActivityPlace::Vehicle(end_place), 1))
                }
                "break" => {
                    let break_place = vehicle_type
                        .vehicle_break
                        .clone()
                        .ok_or(format!("Break activity for vehicle without break: {}", vehicle_id))?;
                    Ok((ActivityPlace::Break(break_place), 0))
                }
                _ => Err(format!("Unknown activity type: {}", self.activity.activity_type)),
            }
        }
    }

    pub fn get_demand(&self) -> Option<&Vec<i32>> {
        unimplemented!()
    }
}
