use crate::MatrixData;
use rosomaxa::algorithms::gsom::{Coordinate, NetworkState};
use rosomaxa::population::{DominanceOrdered, Rosomaxa, RosomaxaWeighted, Shuffled};
use rosomaxa::prelude::*;
use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

/// Represents population state specific for supported types.
#[allow(clippy::large_enum_variant)]
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
#[derive(Default)]
pub struct HyperHeuristicState {
    /// Unique heuristic names.
    pub names: Vec<String>,
    /// Heuristic states at specific generations as (name, estimation, state, duration).
    pub selection_states: HashMap<usize, Vec<(String, f64, String, f64)>>,
    /// Heuristic states at specific generations as (name, estimation, state)
    pub overall_states: HashMap<usize, Vec<(String, f64, String)>>,
}

impl HyperHeuristicState {
    /// Parses multiple heuristic states for all generations at once from raw output.
    pub(crate) fn try_parse_all(data: &str) -> Option<Self> {
        // f.write_fmt(format_args!("name,generation,estimation,state,duration\n"))?;
        if data.starts_with("TELEMETRY") {
            let mut names = HashSet::new();

            let selection_states =
                data.lines().skip(3).take_while(|line| *line != "overall").fold(HashMap::new(), |mut data, line| {
                    let fields: Vec<&str> = line.split(',').collect();
                    let name = fields[0].to_owned();
                    let generation = fields[1].parse().unwrap();

                    names.insert(name.clone());

                    data.entry(generation).or_insert_with(Vec::default).push((
                        name,
                        fields[2].parse().unwrap(),
                        fields[3].to_owned(),
                        fields[4].parse().unwrap(),
                    ));

                    data
                });

            let overall_states =
                data.lines().skip_while(|line| *line != "overall").skip(2).fold(HashMap::new(), |mut data, line| {
                    let fields: Vec<&str> = line.split(',').collect();
                    let name = fields[0].to_owned();
                    let generation = fields[1].parse().unwrap();
                    names.insert(name.clone());

                    data.entry(generation).or_insert_with(Vec::default).push((
                        name,
                        fields[2].parse().unwrap(),
                        fields[3].to_owned(),
                    ));

                    data
                });

            let mut names = names.into_iter().collect::<Vec<_>>();
            names.sort();

            Some(Self { names, selection_states, overall_states })
        } else {
            None
        }
    }
}
