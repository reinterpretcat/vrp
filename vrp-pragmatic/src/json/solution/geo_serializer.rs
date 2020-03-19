use super::Solution;
use crate::json::solution::{Stop, Tour};
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

fn slice_to_map(vec: &[(&str, &str)]) -> HashMap<String, String> {
    vec.into_iter().map(|(key, value)| (key.to_string(), value.to_string())).collect()
}

fn get_marker_symbol(stop: &Stop) -> String {
    let contains_activity_type =
        |activity_type: &str| stop.activities.iter().any(|activity| activity.activity_type == activity_type);
    match (
        contains_activity_type("departure"),
        contains_activity_type("arrival"),
        contains_activity_type("break"),
        contains_activity_type("reload"),
    ) {
        (true, ..) | (_, true, ..) | (.., true) => "warehouse",
        (.., true, _) => "beer",
        _ => "marker",
    }
    .to_string()
}

fn get_stop_point(tour_idx: usize, stop_idx: usize, stop: &Stop, color: &str) -> Feature {
    Feature {
        properties: slice_to_map(&[
            ("marker-color", color),
            ("marker-size", "medium"),
            ("marker-symbol", get_marker_symbol(&stop).as_str()),
            ("tour_idx", tour_idx.to_string().as_str()),
            ("stop_idx", stop_idx.to_string().as_str()),
            ("jobs_ids", stop.activities.iter().map(|a| a.job_id.clone()).collect::<Vec<_>>().join(",").as_str()),
        ]),
        geometry: Geometry::Point { coordinates: (stop.location.lng, stop.location.lat) },
    }
}

fn get_tour_line(tour_idx: usize, tour: &Tour, color: &str) -> Feature {
    Feature {
        properties: slice_to_map(&[
            ("vehicle_id", tour.vehicle_id.as_str()),
            ("tour_idx", tour_idx.to_string().as_str()),
            ("shift_idx", tour.shift_index.to_string().as_str()),
            ("activities:", tour.stops.iter().map(|stop| stop.activities.len()).sum::<usize>().to_string().as_str()),
            ("stroke-width", "4"),
            ("stroke", color),
        ]),
        geometry: Geometry::LineString {
            coordinates: tour.stops.iter().map(|stop| (stop.location.lng, stop.location.lat)).collect(),
        },
    }
}

/// Serializes solution into geo json format.
pub fn serialize_solution_as_geojson<W: Write>(writer: BufWriter<W>, solution: &Solution) -> Result<(), Error> {
    let stop_markers = solution.tours.iter().enumerate().flat_map(|(tour_idx, tour)| {
        tour.stops.iter().enumerate().map(move |(stop_idx, stop)| {
            get_stop_point(tour_idx, stop_idx, &stop, get_color_inverse(tour_idx).as_str())
        })
    });

    let stop_lines = solution
        .tours
        .iter()
        .enumerate()
        .map(|(tour_idx, tour)| get_tour_line(tour_idx, tour, get_color(tour_idx).as_str()));

    serde_json::to_writer_pretty(
        writer,
        &FeatureCollection { features: stop_markers.into_iter().chain(stop_lines.into_iter()).collect() },
    )
}

fn get_color(idx: usize) -> String {
    static COLOR_LIST: ColorList = get_color_list();

    let idx = idx % COLOR_LIST.len();

    COLOR_LIST.get(idx).as_ref().unwrap().1.to_owned()
}

fn get_color_inverse(idx: usize) -> String {
    static COLOR_LIST: ColorList = get_color_list();

    let idx = (COLOR_LIST.len() - idx) % COLOR_LIST.len();

    COLOR_LIST.get(idx).as_ref().unwrap().1.to_owned()
}

type ColorList = &'static [(&'static str, &'static str); 37];

/// Returns list of human distinguishable colors.
const fn get_color_list() -> ColorList {
    &[
        ("aqua", "#00ffff"),
        ("black", "#000000"),
        ("brown", "#a52a2a"),
        ("darkblue", "#00008b"),
        ("darkcyan", "#008b8b"),
        ("darkgrey", "#a9a9a9"),
        ("darkgreen", "#006400"),
        ("darkkhaki", "#bdb76b"),
        ("darkmagenta", "#8b008b"),
        ("darkolivegreen", "#556b2f"),
        ("darkorange", "#ff8c00"),
        ("darkorchid", "#9932cc"),
        ("darkred", "#8b0000"),
        ("darksalmon", "#e9967a"),
        ("darkviolet", "#9400d3"),
        ("fuchsia", "#ff00ff"),
        ("gold", "#ffd700"),
        ("green", "#008000"),
        ("indigo", "#4b0082"),
        ("khaki", "#f0e68c"),
        ("lightblue", "#add8e6"),
        ("lightcyan", "#e0ffff"),
        ("lightgreen", "#90ee90"),
        ("lightgrey", "#d3d3d3"),
        ("lightpink", "#ffb6c1"),
        ("lightyellow", "#ffffe0"),
        ("lime", "#00ff00"),
        ("magenta", "#ff00ff"),
        ("maroon", "#800000"),
        ("navy", "#000080"),
        ("olive", "#808000"),
        ("orange", "#ffa500"),
        ("pink", "#ffc0cb"),
        ("purple", "#800080"),
        ("red", "#ff0000"),
        ("silver", "#c0c0c0"),
        ("yellow", "#ffff00"),
    ]
}
