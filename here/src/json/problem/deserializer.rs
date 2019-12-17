#[cfg(test)]
#[path = "../../../tests/unit/json/problem/deserializer_test.rs"]
mod deserializer_test;

extern crate serde_json;

use self::serde_json::Error;
use crate::json::Location;
use serde::Deserialize;
use std::io::{BufReader, Read};

// region Plan

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RelationType {
    Tour,
    Flexible,
    Sequence,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Relation {
    #[serde(rename(deserialize = "type"))]
    pub type_field: RelationType,
    pub jobs: Vec<String>,
    pub vehicle_id: String,
    pub shift_index: Option<usize>,
}

#[derive(Clone, Deserialize)]
pub struct JobPlace {
    pub times: Option<Vec<Vec<String>>>,
    pub location: Location,
    pub duration: f64,
    pub tag: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct JobPlaces {
    pub pickup: Option<JobPlace>,
    pub delivery: Option<JobPlace>,
}

#[derive(Clone, Deserialize)]
pub struct Job {
    pub id: String,
    pub places: JobPlaces,
    pub demand: Vec<i32>,
    pub skills: Option<Vec<String>>,
}

#[derive(Clone, Deserialize)]
pub struct MultiJobPlace {
    pub times: Option<Vec<Vec<String>>>,
    pub location: Location,
    pub duration: f64,
    pub demand: Vec<i32>,
    pub tag: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct MultiJobPlaces {
    pub pickups: Vec<MultiJobPlace>,
    pub deliveries: Vec<MultiJobPlace>,
}

#[derive(Clone, Deserialize)]
pub struct MultiJob {
    pub id: String,
    pub places: MultiJobPlaces,
    pub skills: Option<Vec<String>>,
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub enum JobVariant {
    Single(Job),
    Multi(MultiJob),
}

#[derive(Clone, Deserialize)]
pub struct Plan {
    pub jobs: Vec<JobVariant>,
    pub relations: Option<Vec<Relation>>,
}

// endregion

// region Fleet

#[derive(Clone, Deserialize)]
pub struct VehicleCosts {
    pub fixed: Option<f64>,
    pub distance: f64,
    pub time: f64,
}

#[derive(Clone, Deserialize)]
pub struct VehiclePlace {
    pub time: String,
    pub location: Location,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleShift {
    pub start: VehiclePlace,
    pub end: Option<VehiclePlace>,
    pub breaks: Option<Vec<VehicleBreak>>,
    pub reloads: Option<Vec<VehicleReload>>,
}

pub type VehicleReload = JobPlace;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VehicleLimits {
    pub max_distance: Option<f64>,
    pub shift_time: Option<f64>,
}

#[derive(Clone, Deserialize)]
pub struct VehicleBreak {
    pub times: Vec<Vec<String>>,
    pub duration: f64,
    pub location: Option<Location>,
}

#[derive(Clone, Deserialize)]
pub struct VehicleType {
    pub id: String,
    pub profile: String,
    pub costs: VehicleCosts,
    pub shifts: Vec<VehicleShift>,
    pub capacity: Vec<i32>,
    pub amount: i32,
    pub skills: Option<Vec<String>>,
    pub limits: Option<VehicleLimits>,
}

#[derive(Clone, Deserialize)]
pub struct Profile {
    pub name: String,
    #[serde(rename(deserialize = "type"))]
    pub profile_type: String,
}

#[derive(Clone, Deserialize)]
pub struct Fleet {
    pub types: Vec<VehicleType>,
    pub profiles: Vec<Profile>,
}

// endregion

// region Configuration

#[derive(Clone, Deserialize)]
pub struct Config {
    pub features: Option<Features>,
}

#[derive(Clone, Deserialize)]
pub struct Features {
    pub even_distribution: Option<EvenDistribution>,
}

#[derive(Clone, Deserialize)]
pub struct EvenDistribution {
    pub enabled: bool,
    pub extra_cost: Option<f64>,
}

// endregion

// region Common

#[derive(Clone, Deserialize)]
pub struct Problem {
    pub id: String,
    pub plan: Plan,
    pub fleet: Fleet,
    pub config: Option<Config>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Matrix {
    pub num_origins: i32,
    pub num_destinations: i32,
    pub travel_times: Vec<i64>,
    pub distances: Vec<i64>,
    pub error_codes: Option<Vec<i64>>,
}

// endregion

pub fn deserialize_problem<R: Read>(reader: BufReader<R>) -> Result<Problem, Error> {
    serde_json::from_reader(reader)
}

pub fn deserialize_matrix<R: Read>(reader: BufReader<R>) -> Result<Matrix, Error> {
    serde_json::from_reader(reader)
}
