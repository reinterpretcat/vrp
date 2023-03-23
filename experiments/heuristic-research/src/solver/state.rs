use crate::MatrixData;
use rosomaxa::algorithms::gsom::{Coordinate, NetworkState};
use rosomaxa::population::{DominanceOrdered, Rosomaxa, RosomaxaWeighted, Shuffled};
use rosomaxa::prelude::*;
use std::any::TypeId;
use std::ops::Range;

/// Represents population state specific for supported types.
#[allow(clippy::large_enum_variant)]
pub enum PopulationState {
    /// Unknown (or unimplemented) population type.
    Unknown,
    /// Rosomaxa type.
    Rosomaxa {
        /// Rows range.
        rows: Range<i32>,
        /// Cols range.
        cols: Range<i32>,
        /// Fitness values data split into separate matrices.
        fitness: Vec<MatrixData>,
        /// Mean distance.
        mean_distance: f64,
        /// U-matrix values data.
        u_matrix: MatrixData,
        /// T-matrix values data.
        t_matrix: MatrixData,
        /// L-matrix values data.
        l_matrix: MatrixData,
        /// Node distance values data.
        n_matrix: MatrixData,
    },
}

impl PopulationState {
    /// Creates a new instance of `PopulationState::Rosomaxa` with empty fields.
    fn new_rosomaxa_empty(rows: Range<i32>, cols: Range<i32>) -> PopulationState {
        PopulationState::Rosomaxa {
            rows,
            cols,
            fitness: Default::default(),
            mean_distance: 0.,
            u_matrix: Default::default(),
            t_matrix: Default::default(),
            l_matrix: Default::default(),
            n_matrix: Default::default(),
        }
    }
}

/// Parses population state from a string representation.
pub fn get_population_state<P, O, S>(population: &P) -> PopulationState
where
    P: HeuristicPopulation<Objective = O, Individual = S> + 'static,
    O: HeuristicObjective<Solution = S> + Shuffled + 'static,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered + 'static,
{
    // TODO try parse elitism and greedy

    if TypeId::of::<P>() == TypeId::of::<Rosomaxa<O, S>>() {
        let rosomaxa = unsafe { std::mem::transmute::<&P, &Rosomaxa<O, S>>(population) };
        NetworkState::try_from(rosomaxa).map(create_rosomaxa_state).unwrap_or(PopulationState::Unknown)
    } else {
        PopulationState::Unknown
    }
}

fn create_rosomaxa_state(network_state: NetworkState) -> PopulationState {
    let (rows, cols, _) = network_state.shape;

    network_state.nodes.iter().fold(PopulationState::new_rosomaxa_empty(rows, cols), |mut rosomaxa, node| {
        let coordinate = Coordinate(node.coordinate.0, node.coordinate.1);
        match &mut rosomaxa {
            PopulationState::Rosomaxa { fitness, mean_distance, u_matrix, t_matrix, l_matrix, n_matrix, .. } => {
                let objectives = fitness;
                // NOTE get first fitness in assumption of sorted order
                let fitness = match (node.dump.starts_with("[["), node.dump.find(']')) {
                    (true, Some(value)) => node.dump[2..value]
                        .split(',')
                        .map(|value| value.parse::<f64>())
                        .collect::<Result<Vec<_>, _>>()
                        .ok(),
                    _ => None,
                };

                if let Some(fitness) = fitness {
                    objectives.resize(fitness.len(), MatrixData::default());
                    fitness.into_iter().enumerate().for_each(|(idx, fitness)| {
                        objectives[idx].insert(coordinate, fitness);
                    });
                }

                if let Some(node_distance) = node.node_distance {
                    n_matrix.insert(coordinate, node_distance);
                }

                u_matrix.insert(coordinate, node.unified_distance);
                t_matrix.insert(coordinate, node.total_hits as f64);
                l_matrix.insert(coordinate, node.last_hits as f64);
                *mean_distance = network_state.mean_distance;
            }
            _ => unreachable!(),
        }

        rosomaxa
    })
}
