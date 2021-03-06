#[cfg(test)]
#[path = "../../../tests/unit/format/solution/geo_serializer_test.rs"]
mod geo_serializer_test;

use super::Solution;
use crate::format::solution::{Stop, Tour, UnassignedJob};
use crate::format::{get_coord_index, get_job_index, CoordIndex, Location};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{BufWriter, Error, ErrorKind, Write};
use vrp_core::models::problem::{Job, Place};
use vrp_core::models::Problem;

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

fn slice_to_map(vec: &[(&str, &str)]) -> HashMap<String, String> {
    vec.iter().map(|&(key, value)| (key.to_string(), value.to_string())).collect()
}

fn get_marker_symbol(stop: &Stop) -> String {
    let contains_activity_type =
        |activity_type: &&str| stop.activities.iter().any(|activity| activity.activity_type == *activity_type);
    match (
        ["departure", "dispatch", "reload", "arrival"].iter().any(contains_activity_type),
        contains_activity_type(&"break"),
    ) {
        (true, _) => "warehouse",
        (_, true) => "beer",
        _ => "marker",
    }
    .to_string()
}

fn get_stop_point(tour_idx: usize, stop_idx: usize, stop: &Stop, color: &str) -> Result<Feature, Error> {
    Ok(Feature {
        properties: slice_to_map(&[
            ("marker-color", color),
            ("marker-size", "medium"),
            ("marker-symbol", get_marker_symbol(&stop).as_str()),
            ("tour_idx", tour_idx.to_string().as_str()),
            ("stop_idx", stop_idx.to_string().as_str()),
            ("arrival", stop.time.arrival.as_str()),
            ("departure", stop.time.departure.as_str()),
            ("jobs_ids", stop.activities.iter().map(|a| a.job_id.clone()).collect::<Vec<_>>().join(",").as_str()),
        ]),
        geometry: Geometry::Point { coordinates: get_lng_lat(&stop.location)? },
    })
}

fn get_unassigned_points(
    coord_index: &CoordIndex,
    unassigned: &UnassignedJob,
    job: &Job,
    color: &str,
) -> Result<Vec<Feature>, Error> {
    let places: Box<dyn Iterator<Item = &Place>> = match job {
        Job::Single(single) => Box::new(single.places.iter()),
        Job::Multi(multi) => Box::new(multi.jobs.iter().flat_map(|single| single.places.iter())),
    };

    places
        .filter_map(|place| place.location.and_then(|l| coord_index.get_by_idx(l)))
        .map(|location| {
            let coordinates = get_lng_lat(&location)?;
            Ok(Feature {
                properties: slice_to_map(&[
                    ("marker-color", color),
                    ("marker-size", "medium"),
                    ("marker-symbol", "roadblock"),
                    ("job_id", unassigned.job_id.as_str()),
                    (
                        "reasons",
                        unassigned
                            .reasons
                            .iter()
                            .map(|reason| format!("{}:{}", reason.code, reason.description))
                            .collect::<Vec<_>>()
                            .join(",")
                            .as_str(),
                    ),
                ]),
                geometry: Geometry::Point { coordinates },
            })
        })
        .collect()
}

fn get_tour_line(tour_idx: usize, tour: &Tour, color: &str) -> Result<Feature, Error> {
    let coordinates = tour.stops.iter().map(|stop| get_lng_lat(&stop.location)).collect::<Result<_, Error>>()?;

    Ok(Feature {
        properties: slice_to_map(&[
            ("vehicle_id", tour.vehicle_id.as_str()),
            ("tour_idx", tour_idx.to_string().as_str()),
            ("shift_idx", tour.shift_index.to_string().as_str()),
            ("activities", tour.stops.iter().map(|stop| stop.activities.len()).sum::<usize>().to_string().as_str()),
            ("distance", (tour.stops.last().unwrap().distance).to_string().as_str()),
            ("departure", tour.stops.first().unwrap().time.departure.as_str()),
            ("arrival", tour.stops.last().unwrap().time.arrival.as_str()),
            ("stroke-width", "4"),
            ("stroke", color),
        ]),
        geometry: Geometry::LineString { coordinates },
    })
}

/// Creates solution as geo json.
fn create_geojson_solution(problem: &Problem, solution: &Solution) -> Result<FeatureCollection, Error> {
    let stop_markers = solution
        .tours
        .iter()
        .enumerate()
        .flat_map(|(tour_idx, tour)| {
            tour.stops.iter().enumerate().map(move |(stop_idx, stop)| {
                get_stop_point(tour_idx, stop_idx, &stop, get_color_inverse(tour_idx).as_str())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let stop_lines = solution
        .tours
        .iter()
        .enumerate()
        .map(|(tour_idx, tour)| get_tour_line(tour_idx, tour, get_color(tour_idx).as_str()))
        .collect::<Result<Vec<_>, _>>()?;

    let job_index = get_job_index(problem);
    let coord_index = get_coord_index(problem);
    let unassigned_markers = solution
        .unassigned
        .iter()
        .flat_map(|unassigned| unassigned.iter())
        .enumerate()
        .map(|(idx, unassigned_job)| {
            let job = job_index.get(&unassigned_job.job_id).ok_or_else(|| {
                Error::new(ErrorKind::InvalidData, format!("cannot find job: {}", unassigned_job.job_id))
            })?;
            let color = get_color(idx);
            get_unassigned_points(coord_index, unassigned_job, job, color.as_str())
        })
        .collect::<Result<Vec<Vec<Feature>>, Error>>()?
        .into_iter()
        .flatten();

    Ok(FeatureCollection {
        features: stop_markers.into_iter().chain(stop_lines.into_iter()).chain(unassigned_markers).collect(),
    })
}

/// Serializes solution into geo json format.
pub fn serialize_solution_as_geojson<W: Write>(
    writer: BufWriter<W>,
    problem: &Problem,
    solution: &Solution,
) -> Result<(), Error> {
    let geo_json = create_geojson_solution(problem, solution)?;

    serde_json::to_writer_pretty(writer, &geo_json).map_err(Error::from)
}

fn get_color(idx: usize) -> String {
    static COLOR_LIST: ColorList = get_color_list();

    let idx = idx % COLOR_LIST.len();

    (**COLOR_LIST.get(idx).as_ref().unwrap()).to_string()
}

fn get_color_inverse(idx: usize) -> String {
    static COLOR_LIST: ColorList = get_color_list();

    let idx = (COLOR_LIST.len() - idx) % COLOR_LIST.len();

    (**COLOR_LIST.get(idx).as_ref().unwrap()).to_string()
}

fn get_lng_lat(location: &Location) -> Result<(f64, f64), Error> {
    match location {
        Location::Coordinate { lat, lng } => Ok((*lng, *lat)),
        Location::Reference { index: _ } => {
            Err(Error::new(ErrorKind::InvalidData, "geojson cannot be used with location indices"))
        }
    }
}

type ColorList = &'static [&'static str; 15];

/// Returns list of human distinguishable colors.
const fn get_color_list() -> ColorList {
    &[
        "#e6194b", "#3cb44b", "#4363d8", "#f58231", "#911eb4", "#46f0f0", "#f032e6", "#bcf60c", "#008080", "#e6beff",
        "#9a6324", "#800000", "#808000", "#000075", "#808080",
    ]
}
