use crate::{Coordinate, MatrixData};
use rosomaxa::algorithms::gsom::NetworkState;
use rosomaxa::population::{DominanceOrdered, Rosomaxa, RosomaxaWeighted, Shuffled};
use rosomaxa::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::collections::HashMap;
use std::ops::Range;

/// Represents population state specific for supported types.
#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize)]
pub enum PopulationState {
    /// Unknown (or unimplemented) population type.
    Unknown {
        /// Best fitness values.
        fitness_values: Vec<f64>,
    },
    /// Rosomaxa type.
    Rosomaxa {
        /// Rows range.
        rows: Range<i32>,
        /// Cols range.
        cols: Range<i32>,
        /// Mean distance.
        mean_distance: f64,
        /// Best fitness values.
        fitness_values: Vec<f64>,
        /// Overall fitness values data split into separate matrices.
        fitness_matrices: Vec<MatrixData>,
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

/// Parses population state from a string representation.
pub fn get_population_state<P, O, S>(population: &P) -> PopulationState
where
    P: HeuristicPopulation<Objective = O, Individual = S> + 'static,
    O: HeuristicObjective<Solution = S> + Shuffled + 'static,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered + 'static,
{
    let fitness_values =
        population.ranked().next().map(|(solution, _)| solution.fitness().collect::<Vec<_>>()).unwrap_or_default();

    if TypeId::of::<P>() == TypeId::of::<Rosomaxa<O, S>>() {
        let rosomaxa = unsafe { std::mem::transmute::<&P, &Rosomaxa<O, S>>(population) };
        NetworkState::try_from(rosomaxa)
            .map(|state| create_rosomaxa_state(state, fitness_values.clone()))
            .unwrap_or_else(move |_| PopulationState::Unknown { fitness_values })
    } else {
        PopulationState::Unknown { fitness_values }
    }
}

fn create_rosomaxa_state(network_state: NetworkState, fitness_values: Vec<f64>) -> PopulationState {
    let (rows, cols, _) = network_state.shape;

    let rosomaxa = PopulationState::Rosomaxa {
        rows,
        cols,
        mean_distance: 0.,
        fitness_values,
        fitness_matrices: Default::default(),
        u_matrix: Default::default(),
        t_matrix: Default::default(),
        l_matrix: Default::default(),
        n_matrix: Default::default(),
    };

    network_state.nodes.iter().fold(rosomaxa, |mut rosomaxa, node| {
        let coordinate = Coordinate(node.coordinate.0, node.coordinate.1);
        match &mut rosomaxa {
            PopulationState::Rosomaxa {
                fitness_matrices,
                mean_distance,
                u_matrix,
                t_matrix,
                l_matrix,
                n_matrix,
                ..
            } => {
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
                    fitness_matrices.resize(fitness.len(), MatrixData::default());
                    fitness.into_iter().enumerate().for_each(|(idx, fitness)| {
                        fitness_matrices[idx].insert(coordinate, fitness);
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

/// Keeps track of dynamic selective hyper heuristic state.
#[derive(Default, Serialize, Deserialize)]
pub struct HyperHeuristicState {
    /// Unique heuristic names.
    pub names: HashMap<String, usize>,
    /// Unique state names.
    pub states: HashMap<String, usize>,
    /// Heuristic max estimate
    pub max_estimate: f64,
    /// Heuristic states at specific generations as (name idx, estimation, state idx, duration).
    pub selection_states: HashMap<usize, Vec<(usize, f64, usize, f64)>>,
    /// Heuristic states at specific generations as (name idx, estimation, state idx)
    pub overall_states: HashMap<usize, Vec<(usize, f64, usize)>>,
}

impl HyperHeuristicState {
    /// Parses multiple heuristic states for all generations at once from raw output.
    pub(crate) fn try_parse_all(data: &str) -> Option<Self> {
        if data.starts_with("TELEMETRY") {
            let mut names = HashMap::new();
            let mut states = HashMap::new();
            let mut max_estimate = 0_f64;

            let insert_to_map = |map: &mut HashMap<String, usize>, key: String| {
                let length = map.len();
                map.entry(key).or_insert_with(|| length);
            };

            let mut get_data = |line: &str| {
                let fields: Vec<String> = line.split(',').map(|s| s.to_string()).collect();
                let name = fields[0].clone();
                let generation = fields[1].parse().unwrap();
                let estimate = fields[2].parse().unwrap();
                let state = fields[3].clone();

                insert_to_map(&mut names, name.clone());
                insert_to_map(&mut states, state.clone());

                max_estimate = max_estimate.max(estimate);

                (
                    fields,
                    (names.get(&name).copied().unwrap(), generation, estimate, states.get(&state).copied().unwrap()),
                )
            };

            // TODO sort by name?

            let mut selection_states =
                data.lines().skip(3).take_while(|line| *line != "overall").fold(HashMap::new(), |mut data, line| {
                    let (fields, (name, generation, estimate, state)) = get_data(line);
                    let duration = fields[4].parse().unwrap();

                    data.entry(generation).or_insert_with(Vec::default).push((name, estimate, state, duration));

                    data
                });
            selection_states.values_mut().for_each(|states| states.sort_by(|(a, ..), (b, ..)| a.cmp(b)));

            let mut overall_states =
                data.lines().skip_while(|line| *line != "overall").skip(2).fold(HashMap::new(), |mut data, line| {
                    let (_, (name, generation, estimate, state)) = get_data(line);

                    data.entry(generation).or_insert_with(Vec::default).push((name, estimate, state));

                    data
                });
            overall_states.values_mut().for_each(|states| states.sort_by(|(a, ..), (b, ..)| a.cmp(b)));

            Some(Self { names, states, max_estimate, selection_states, overall_states })
        } else {
            None
        }
    }
}
