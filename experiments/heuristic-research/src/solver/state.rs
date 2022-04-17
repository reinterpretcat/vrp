use crate::MatrixData;
use rosomaxa::algorithms::gsom::{Coordinate, NetworkState};
use std::ops::Range;

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
    if let Ok(network) = NetworkState::try_from(serialized) {
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
