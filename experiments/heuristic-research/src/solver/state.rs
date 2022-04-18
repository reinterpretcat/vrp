#[cfg(test)]
#[path = "../../tests/unit/solver/state_test.rs"]
mod state_test;

use crate::MatrixData;
use regex::{Captures, Regex};
use rosomaxa::algorithms::gsom::{Coordinate, NetworkState, NodeState};
use std::ops::Range;
use std::str::FromStr;

/// Represents population state specific for supported types.
pub enum PopulationState {
    /// Unknown (or unimplemented) population type.
    Unknown,
    /// Rosomaxa type.
    Rosomaxa {
        /// Rows range.
        rows: Range<i32>,
        /// Cols range.
        cols: Range<i32>,
        /// Objective values data.
        objective: MatrixData,
        /// U-matrix values data.
        u_matrix: MatrixData,
        /// T-matrix values data.
        t_matrix: MatrixData,
        /// L-matrix values data.
        l_matrix: MatrixData,
    },
}

impl PopulationState {
    /// Creates a new instance of `PopulationState::Rosomaxa` with empty fields.
    fn new_rosomaxa_empty(rows: Range<i32>, cols: Range<i32>) -> PopulationState {
        PopulationState::Rosomaxa {
            rows,
            cols,
            objective: Default::default(),
            u_matrix: Default::default(),
            t_matrix: Default::default(),
            l_matrix: Default::default(),
        }
    }
}

/// Parses population state from a string representation.
pub fn parse_population_state(serialized: String) -> PopulationState {
    if let Some(network) = try_parse_network_state(&serialized) {
        return create_rosomaxa_state(network);
    }

    // TODO try parse elitism and greedy

    return PopulationState::Unknown;
}

fn create_rosomaxa_state(network_state: NetworkState) -> PopulationState {
    let (rows, cols, num_of_weights) = network_state.shape;

    // NOTE: expecting x, y and objective value
    assert_eq!(num_of_weights, 3);

    network_state.nodes.iter().fold(PopulationState::new_rosomaxa_empty(rows, cols), |mut rosomaxa, node| {
        let coordinate = Coordinate(node.coordinate.0, node.coordinate.1);
        match &mut rosomaxa {
            PopulationState::Rosomaxa { objective, u_matrix, t_matrix, l_matrix, .. } => {
                // NOTE assumption is that last weight is objective value
                objective.insert(coordinate, *node.weights.last().unwrap());
                u_matrix.insert(coordinate, node.unified_distance);
                t_matrix.insert(coordinate, node.total_hits as f64);
                l_matrix.insert(coordinate, node.last_hits as f64);
            }
            _ => unreachable!(),
        }

        rosomaxa
    })
}

fn try_parse_network_state(value: &String) -> Option<NetworkState> {
    lazy_static! {
        static ref NETWORK_STATE_META: Regex = Regex::new(
            r"(?x)\(
                (?P<rows_start>-?\d+),
                (?P<rows_end>-?\d+),
                (?P<cols_start>-?\d+),
                (?P<cols_end>-?\d+),
                (?P<num_weights>-?\d+),
                \[(?P<nodes>.*)\]
            \)"
        )
        .unwrap();
        static ref NETWORK_STATE_NODES: Regex = Regex::new(
            r"(?x)\(
                (?P<x>-?\d+),
                (?P<y>-?\d+),
                (?P<unified_dist>\d*[.]?\d+),
                (?P<total_hits>\d+),
                (?P<last_hits>\d+),
                \[[^,]*,[^,]*,(?P<objective>-?\d*[.]?\d+)],
            \)"
        )
        .unwrap();
    }

    NETWORK_STATE_META.captures(value).map(|captures: Captures| {
        let rows_start = get_captured_value("rows_start", &captures).unwrap();
        let rows_end = get_captured_value("rows_end", &captures).unwrap();
        let cols_start = get_captured_value("cols_start", &captures).unwrap();
        let cols_end = get_captured_value("cols_end", &captures).unwrap();
        let num_weights = get_captured_value("num_weights", &captures).unwrap();

        let nodes = NETWORK_STATE_NODES
            .captures_iter(captures.name("nodes").unwrap().as_str())
            .map(|captures| NodeState {
                coordinate: (get_captured_value("x", &captures).unwrap(), get_captured_value("y", &captures).unwrap()),
                unified_distance: get_captured_value("unified_dist", &captures).unwrap(),
                weights: vec![0., 0., get_captured_value("objective", &captures).unwrap()],
                total_hits: get_captured_value("total_hits", &captures).unwrap(),
                last_hits: get_captured_value("last_hits", &captures).unwrap(),
                dump: "".to_string(),
            })
            .collect();

        NetworkState { shape: ((rows_start..rows_end), (cols_start..cols_end), num_weights), nodes }
    })
}

fn get_captured_value<T>(group: &str, captures: &Captures) -> Result<T, T::Err>
where
    T: FromStr,
{
    captures.name(group).unwrap().as_str().parse::<T>()
}
