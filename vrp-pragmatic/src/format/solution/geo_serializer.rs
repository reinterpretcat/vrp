#[cfg(test)]
#[path = "../../../tests/unit/format/solution/geo_serializer_test.rs"]
mod geo_serializer_test;

use super::Solution;
use crate::format::solution::{Activity, Stop, Tour, UnassignedJob};
use crate::format::{get_coord_index, get_job_index, CoordIndex, Location};
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::{BufWriter, Error, ErrorKind, Write};
use vrp_core::models::problem::Job;
use vrp_core::models::Problem;
use vrp_core::utils::compare_floats;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
enum Geometry {
    Point { coordinates: (f64, f64) },
    LineString { coordinates: Vec<(f64, f64)> },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
struct Feature {
    pub properties: HashMap<String, String>,
    pub geometry: Geometry,
}

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
#[serde(tag = "type")]
struct FeatureCollection {
    pub features: Vec<Feature>,
}

impl Eq for Geometry {}

impl PartialEq for Geometry {
    fn eq(&self, other: &Self) -> bool {
        let compare_pair = |l_coord: &(f64, f64), r_coord: &(f64, f64)| {
            compare_floats(l_coord.0, r_coord.0) == Ordering::Equal
                && compare_floats(l_coord.1, r_coord.1) == Ordering::Equal
        };

        match (self, other) {
            (Geometry::Point { coordinates: l_coord }, Geometry::Point { coordinates: r_coord }) => {
                compare_pair(l_coord, r_coord)
            }
            (Geometry::LineString { coordinates: l_coords }, Geometry::LineString { coordinates: r_coords }) => {
                l_coords.len() == r_coords.len()
                    && l_coords.iter().zip(r_coords.iter()).all(|(l_coord, r_coord)| compare_pair(l_coord, r_coord))
            }
            _ => false,
        }
    }
}

impl Eq for Feature {}

impl PartialEq for Feature {
    fn eq(&self, other: &Self) -> bool {
        let same_properties = self.properties.len() == other.properties.len()
            && self.properties.keys().all(|key| {
                if let Some(value) = other.properties.get(key) {
                    self.properties[key] == *value
                } else {
                    false
                }
            });

        same_properties && self.geometry.eq(&other.geometry)
    }
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

/// Serializes named location list with their color index.
pub fn serialize_named_locations_as_geojson<W: Write>(
    writer: BufWriter<W>,
    locations: &[(String, Location, usize)],
) -> Result<(), Error> {
    let geo_json = create_geojson_named_locations(locations)?;

    serde_json::to_writer_pretty(writer, &geo_json).map_err(Error::from)
}

fn create_geojson_named_locations(locations: &[(String, Location, usize)]) -> Result<FeatureCollection, Error> {
    let colors = get_more_colors();

    Ok(FeatureCollection {
        features: locations
            .iter()
            .map(|(name, location, index)| {
                Ok(Feature {
                    properties: slice_to_map(&[
                        ("marker-color", colors[*index % colors.len()]),
                        ("marker-size", "medium"),
                        ("marker-symbol", "marker"),
                        ("name", name),
                    ]),
                    geometry: Geometry::Point { coordinates: get_lng_lat(location)? },
                })
            })
            .collect::<Result<Vec<_>, Error>>()?,
    })
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
    // TODO add parking
    Ok(Feature {
        properties: slice_to_map(&[
            ("marker-color", color),
            ("marker-size", "medium"),
            ("marker-symbol", get_marker_symbol(stop).as_str()),
            ("tour_idx", tour_idx.to_string().as_str()),
            ("stop_idx", stop_idx.to_string().as_str()),
            ("arrival", stop.time.arrival.as_str()),
            ("departure", stop.time.departure.as_str()),
            ("jobs_ids", stop.activities.iter().map(|a| a.job_id.clone()).collect::<Vec<_>>().join(",").as_str()),
        ]),
        geometry: Geometry::Point { coordinates: get_lng_lat(&stop.location)? },
    })
}

fn get_activity_point(
    tour_idx: usize,
    stop_idx: usize,
    activity_idx: usize,
    activity: &Activity,
    location: &Location,
    color: &str,
) -> Result<Feature, Error> {
    let time =
        activity.time.as_ref().ok_or_else(|| Error::new(ErrorKind::InvalidData, "activity has no time defined"))?;

    Ok(Feature {
        properties: slice_to_map(&[
            ("marker-color", color),
            ("marker-size", "medium"),
            ("marker-symbol", "marker"),
            ("tour_idx", tour_idx.to_string().as_str()),
            ("stop_idx", stop_idx.to_string().as_str()),
            ("activity_idx", activity_idx.to_string().as_str()),
            ("start", time.start.as_str()),
            ("end", time.end.as_str()),
            ("jobs_id", activity.job_id.as_str()),
        ]),
        geometry: Geometry::Point { coordinates: get_lng_lat(location)? },
    })
}

fn get_cluster_geometry(tour_idx: usize, stop_idx: usize, stop: &Stop) -> Result<Vec<Feature>, Error> {
    let features = stop.activities.iter().enumerate().try_fold::<_, _, Result<_, Error>>(
        Vec::<Feature>::new(),
        |mut features, (activity_idx, activity)| {
            let location = activity.location.clone().ok_or_else(|| invalid_data("activity without location"))?;
            features.push(get_activity_point(
                tour_idx,
                stop_idx,
                activity_idx,
                activity,
                &location,
                get_color(tour_idx).as_str(),
            )?);

            let line_color = get_color_inverse(tour_idx);
            let get_line = |from: (f64, f64), to: (f64, f64)| -> Feature {
                Feature {
                    properties: slice_to_map(&[("stroke-width", "3"), ("stroke", line_color.as_str())]),
                    geometry: Geometry::LineString { coordinates: vec![from, to] },
                }
            };

            if let Some(commute) = &activity.commute {
                if let Some(forward) = &commute.forward {
                    features.push(get_line(get_lng_lat(&forward.location)?, get_lng_lat(&location)?));
                }

                if let Some(backward) = &commute.backward {
                    features.push(get_line(get_lng_lat(&location)?, get_lng_lat(&backward.location)?));
                }
            }

            Ok(features)
        },
    )?;

    Ok(features)
}

fn get_unassigned_points(
    coord_index: &CoordIndex,
    unassigned: &UnassignedJob,
    job: &Job,
    color: &str,
) -> Result<Vec<Feature>, Error> {
    job.places()
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
                get_stop_point(tour_idx, stop_idx, stop, get_color_inverse(tour_idx).as_str())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let clusters_geometry = solution
        .tours
        .iter()
        .enumerate()
        .flat_map(|(tour_idx, tour)| {
            tour.stops
                .iter()
                .enumerate()
                .filter(|(_, stop)| stop.parking.is_some())
                .map(move |(stop_idx, stop)| get_cluster_geometry(tour_idx, stop_idx, stop))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten();

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
            let job = job_index
                .get(&unassigned_job.job_id)
                .ok_or_else(|| invalid_data(format!("cannot find job: {}", unassigned_job.job_id).as_str()))?;
            let color = get_color(idx);
            get_unassigned_points(coord_index, unassigned_job, job, color.as_str())
        })
        .collect::<Result<Vec<Vec<Feature>>, Error>>()?
        .into_iter()
        .flatten();

    Ok(FeatureCollection {
        features: stop_markers
            .into_iter()
            .chain(stop_lines.into_iter())
            .chain(unassigned_markers)
            .chain(clusters_geometry)
            .collect(),
    })
}

fn get_color(idx: usize) -> String {
    static COLOR_LIST: ColorList = get_color_list();

    let idx = idx % COLOR_LIST.len();

    (**COLOR_LIST.get(idx).as_ref().unwrap()).to_string()
}

fn get_color_inverse(idx: usize) -> String {
    static COLOR_LIST: ColorList = get_color_list();

    let idx = (COLOR_LIST.len() - idx + 1) % COLOR_LIST.len();

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

fn invalid_data(msg: &str) -> Error {
    Error::new(ErrorKind::InvalidData, msg)
}

type ColorList = &'static [&'static str; 15];

/// Returns list of human distinguishable colors.
const fn get_color_list() -> ColorList {
    &[
        "#e6194b", "#3cb44b", "#4363d8", "#f58231", "#911eb4", "#46f0f0", "#f032e6", "#bcf60c", "#008080", "#e6beff",
        "#9a6324", "#800000", "#808000", "#000075", "#808080",
    ]
}

type MoreColorList = &'static [&'static str; 128];

/// Returns more colors.
const fn get_more_colors() -> MoreColorList {
    &[
        "#000000", "#FFFF00", "#1CE6FF", "#FF34FF", "#FF4A46", "#008941", "#006FA6", "#A30059", "#FFDBE5", "#7A4900",
        "#0000A6", "#63FFAC", "#B79762", "#004D43", "#8FB0FF", "#997D87", "#5A0007", "#809693", "#FEFFE6", "#1B4400",
        "#4FC601", "#3B5DFF", "#4A3B53", "#FF2F80", "#61615A", "#BA0900", "#6B7900", "#00C2A0", "#FFAA92", "#FF90C9",
        "#B903AA", "#D16100", "#DDEFFF", "#000035", "#7B4F4B", "#A1C299", "#300018", "#0AA6D8", "#013349", "#00846F",
        "#372101", "#FFB500", "#C2FFED", "#A079BF", "#CC0744", "#C0B9B2", "#C2FF99", "#001E09", "#00489C", "#6F0062",
        "#0CBD66", "#EEC3FF", "#456D75", "#B77B68", "#7A87A1", "#788D66", "#885578", "#FAD09F", "#FF8A9A", "#D157A0",
        "#BEC459", "#456648", "#0086ED", "#886F4C", "#34362D", "#B4A8BD", "#00A6AA", "#452C2C", "#636375", "#A3C8C9",
        "#FF913F", "#938A81", "#575329", "#00FECF", "#B05B6F", "#8CD0FF", "#3B9700", "#04F757", "#C8A1A1", "#1E6E00",
        "#7900D7", "#A77500", "#6367A9", "#A05837", "#6B002C", "#772600", "#D790FF", "#9B9700", "#549E79", "#FFF69F",
        "#201625", "#72418F", "#BC23FF", "#99ADC0", "#3A2465", "#922329", "#5B4534", "#FDE8DC", "#404E55", "#0089A3",
        "#CB7E98", "#A4E804", "#324E72", "#6A3A4C", "#83AB58", "#001C1E", "#D1F7CE", "#004B28", "#C8D0F6", "#A3A489",
        "#806C66", "#222800", "#BF5650", "#E83000", "#66796D", "#DA007C", "#FF1A59", "#8ADBB4", "#1E0200", "#5B4E51",
        "#C895C5", "#320033", "#FF6832", "#66E1D3", "#CFCDAC", "#D0AC94", "#7ED379", "#012C58",
    ]
}
