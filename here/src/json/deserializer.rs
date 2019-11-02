#[cfg(test)]
#[path = "../../tests/unit/json/deserializer_test.rs"]
mod deserializer_test;

extern crate serde_json;

use self::serde_json::Error;
use serde::Deserialize;
use std::io::{BufReader, Read};

// region Plan

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum RelationType {
    Tour,
    Flexible,
    Sequence,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Relation {
    #[serde(rename(deserialize = "type"))]
    pub type_field: RelationType,
    pub jobs: Vec<String>,
    pub vehicle_id: Vec<String>,
}

#[derive(Deserialize)]
struct JobPlace {
    pub times: Option<Vec<Vec<String>>>,
    pub location: Vec<f64>,
    pub duration: f64,
    pub tag: Option<String>,
}

#[derive(Deserialize)]
struct JobPlaces {
    pub pickup: Option<JobPlace>,
    pub delivery: Option<JobPlace>,
}

#[derive(Deserialize)]
struct Job {
    pub id: String,
    pub places: JobPlaces,
    pub demand: Vec<i32>,
    pub skills: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct MultiJobPlace {
    pub times: Option<Vec<Vec<String>>>,
    pub location: Vec<f64>,
    pub duration: f64,
    pub demand: Vec<i32>,
    pub tag: Option<String>,
}

#[derive(Deserialize)]
struct MultiJobPlaces {
    pickups: Vec<MultiJobPlace>,
    deliveries: Vec<MultiJobPlace>,
}

#[derive(Deserialize)]
struct MultiJob {
    pub id: String,
    pub places: MultiJobPlaces,
    pub skills: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum JobVariant {
    Single(Job),
    Multi(MultiJob),
}

#[derive(Deserialize)]
struct Plan {
    pub jobs: Vec<JobVariant>,
    pub relations: Option<Vec<Relation>>,
}

// endregion

// region Fleet

#[derive(Deserialize)]
struct VehicleCosts {
    pub fixed: Option<f64>,
    pub distance: f64,
    pub time: f64,
}

#[derive(Deserialize)]
struct VehiclePlace {
    pub time: String,
    pub location: Vec<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VehiclePlaces {
    pub start: VehiclePlace,
    pub end: Option<VehiclePlace>,
    pub max_tours: Option<i32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VehicleLimits {
    pub max_distance: Option<f64>,
    pub shift_time: Option<f64>,
}

#[derive(Deserialize)]
struct VehicleBreak {
    pub times: Vec<Vec<String>>,
    pub duration: f64,
    pub location: Option<Vec<f64>>,
}

#[derive(Deserialize)]
struct VehicleType {
    pub id: String,
    pub profile: String,
    pub costs: VehicleCosts,
    pub places: VehiclePlaces,
    pub capacity: Vec<i32>,
    pub amount: i32,

    pub skills: Option<Vec<String>>,
    pub limits: Option<VehicleLimits>,
    #[serde(rename(deserialize = "break"))]
    pub vehicle_break: Option<VehicleBreak>,
}

#[derive(Deserialize)]
struct Fleet {
    pub types: Vec<VehicleType>,
}

// endregion

// region Common

#[derive(Deserialize)]
struct Problem {
    pub id: String,
    pub plan: Plan,
    pub fleet: Fleet,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Matrix {
    pub num_origins: i32,
    pub num_destinations: i32,
    pub travel_times: Vec<i64>,
    pub distances: Vec<i64>,
    pub error_codes: Option<Vec<i64>>,
}

// endregion

fn deserialize_problem<R: Read>(reader: BufReader<R>) -> Result<Problem, Error> {
    serde_json::from_reader(reader)
}

fn deserialize_matrix<R: Read>(reader: BufReader<R>) -> Result<Matrix, Error> {
    serde_json::from_reader(reader)
}
