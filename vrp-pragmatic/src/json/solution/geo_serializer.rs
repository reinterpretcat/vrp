use super::Solution;
use serde::Serialize;
use serde_json::Error;
use std::collections::HashMap;
use std::io::{BufWriter, Write};

#[derive(Clone, Serialize, Debug)]
#[serde(tag = "type")]
enum Geometry {
    Point { coordinates: (f64, f64) },
    LineString { coordinates: Vec<(f64, f64)> },
}

#[derive(Clone, Serialize, Debug)]
#[serde(tag = "type")]
struct Feature {
    pub properties: HashMap<String, String>,
    pub geometry: Geometry,
}

#[derive(Clone, Serialize, Debug)]
#[serde(tag = "type")]
struct FeatureCollection {
    pub features: Vec<Feature>,
}

/// Serializes solution into geo json format.
pub fn serialize_solution_as_geojson<W: Write>(writer: BufWriter<W>, solution: &Solution) -> Result<(), Error> {
    let stop_markers = solution.tours.iter().map(|_tour| unimplemented!());

    let stop_lines = solution.tours.iter().map(|_tour| unimplemented!());

    serde_json::to_writer_pretty(
        writer,
        &FeatureCollection { features: stop_markers.into_iter().chain(stop_lines.into_iter()).collect() },
    )
}
