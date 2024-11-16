use crate::{Coordinate, MatrixData};
use rosomaxa::algorithms::gsom::NetworkState;
use rosomaxa::population::{Rosomaxa, RosomaxaWeighted, Shuffled};
use rosomaxa::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::collections::HashMap;
use std::ops::Range;
use vrp_scientific::core::models::common::{Footprint, Shadow};

/// Represents population state specific for supported types.
#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize)]
pub enum PopulationState {
    /// Unknown (or unimplemented) population type.
    Unknown {
        /// Best fitness values.
        fitness_values: Vec<Float>,
    },
    /// Rosomaxa type.
    Rosomaxa {
        /// Rows range.
        rows: Range<i32>,
        /// Cols range.
        cols: Range<i32>,
        /// Mean distance.
        mean_distance: Float,
        /// Best fitness values.
        fitness_values: Vec<Float>,
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
    S: HeuristicSolution + RosomaxaWeighted + 'static,
{
    let fitness_values =
        population.ranked().next().map(|solution| solution.fitness().collect::<Vec<_>>()).unwrap_or_default();

    if TypeId::of::<P>() == TypeId::of::<Rosomaxa<O, S>>() {
        let rosomaxa = unsafe { std::mem::transmute::<&P, &Rosomaxa<O, S>>(population) };
        NetworkState::try_from(rosomaxa)
            .map(|state| create_rosomaxa_state(state, fitness_values.clone()))
            .unwrap_or_else(move |_| PopulationState::Unknown { fitness_values })
    } else {
        PopulationState::Unknown { fitness_values }
    }
}

fn create_rosomaxa_state(network_state: NetworkState, fitness_values: Vec<Float>) -> PopulationState {
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
                        .map(|value| value.parse::<Float>())
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
                t_matrix.insert(coordinate, node.total_hits as Float);
                l_matrix.insert(coordinate, node.last_hits as Float);
                *mean_distance = network_state.mean_distance;
            }
            _ => unreachable!(),
        }

        rosomaxa
    })
}

/// Search state result represented as (name idx, reward, (from state idx, to state idx), duration).
#[derive(Default, Serialize, Deserialize)]
pub struct SearchResult(pub usize, pub Float, pub (usize, usize), pub usize);

/// Heuristic state result represented as (state idx, name idx, alpha, beta, mu, v, n).
#[derive(Default, Serialize, Deserialize)]
pub struct HeuristicResult(pub usize, pub usize, pub Float, pub Float, pub Float, pub Float, pub usize);

/// Keeps track of dynamic selective hyper heuristic state.
#[derive(Default, Serialize, Deserialize)]
pub struct HyperHeuristicState {
    /// Unique heuristic names.
    pub names: HashMap<String, usize>,
    /// Unique state names.
    pub states: HashMap<String, usize>,
    /// Search states at specific generations.
    pub search_states: HashMap<usize, Vec<SearchResult>>,
    /// Heuristic states at specific generations.
    pub heuristic_states: HashMap<usize, Vec<HeuristicResult>>,
}

impl HyperHeuristicState {
    /// Parses multiple heuristic states for all generations at once from raw output.
    pub(crate) fn try_parse_all(data: &str) -> Option<Self> {
        if data.starts_with("TELEMETRY") {
            let mut names = HashMap::new();
            let mut states = HashMap::new();

            let insert_to_map = |map: &mut HashMap<String, usize>, key: String| {
                let length = map.len();
                map.entry(key).or_insert_with(|| length);
            };

            let mut search_states = data.lines().skip(3).take_while(|line| *line != "heuristic:").fold(
                HashMap::<_, Vec<_>>::new(),
                |mut data, line| {
                    let fields: Vec<String> = line.split(',').map(|s| s.to_string()).collect();
                    let name = fields[0].clone();
                    let generation = fields[1].parse().unwrap();
                    let reward = fields[2].parse().unwrap();
                    let from = fields[3].clone();
                    let to = fields[4].clone();
                    let duration = fields[5].parse().unwrap();

                    insert_to_map(&mut names, name.clone());
                    insert_to_map(&mut states, from.clone());
                    insert_to_map(&mut states, to.clone());

                    let name = names.get(&name).copied().unwrap();
                    let from = states.get(&from).copied().unwrap();
                    let to = states.get(&to).copied().unwrap();

                    data.entry(generation).or_default().push(SearchResult(name, reward, (from, to), duration));

                    data
                },
            );
            search_states
                .values_mut()
                .for_each(|states| states.sort_by(|SearchResult(a, ..), SearchResult(b, ..)| a.cmp(b)));

            let mut heuristic_states = data.lines().skip_while(|line| *line != "heuristic:").skip(2).fold(
                HashMap::<_, Vec<_>>::new(),
                |mut data, line| {
                    let fields: Vec<String> = line.split(',').map(|s| s.to_string()).collect();

                    let generation: usize = fields[0].parse().unwrap();
                    let state = fields[1].clone();
                    let name = fields[2].clone();
                    let alpha = fields[3].parse().unwrap();
                    let beta = fields[4].parse().unwrap();
                    let mu = fields[5].parse().unwrap();
                    let v = fields[6].parse().unwrap();
                    let n = fields[7].parse().unwrap();

                    insert_to_map(&mut states, state.clone());
                    insert_to_map(&mut names, name.clone());

                    let state = states.get(&state).copied().unwrap();
                    let name = names.get(&name).copied().unwrap();

                    data.entry(generation).or_default().push(HeuristicResult(state, name, alpha, beta, mu, v, n));

                    data
                },
            );
            heuristic_states
                .values_mut()
                .for_each(|states| states.sort_by(|HeuristicResult(_, a, ..), HeuristicResult(_, b, ..)| a.cmp(b)));

            Some(Self { names, states, search_states, heuristic_states })
        } else {
            None
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct FootprintState {
    // TODO find a way to serialize/deserialize state efficiently
    #[serde(skip)]
    repr: HashMap<(usize, usize), u8>,
    dimension: usize,
}

impl FootprintState {
    pub fn apply(&mut self, shadow_state: &ShadowState) {
        shadow_state.shadow.iter().flat_map(|shadow| shadow.iter()).for_each(|((from, to), bit)| {
            self.repr
                .entry((from, to))
                .and_modify(|value| *value = value.saturating_add(bit as u8))
                .or_insert(bit as u8);
        })
    }

    pub fn get(&self, from: usize, to: usize) -> u8 {
        self.repr.get(&(from, to)).copied().unwrap_or_default()
    }
}

impl From<&Footprint> for FootprintState {
    fn from(footprint: &Footprint) -> Self {
        Self { repr: footprint.iter().collect(), dimension: footprint.dimension() }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct ShadowState {
    // NOTE use original shadow as more space efficient representation.
    #[serde(skip)]
    shadow: Option<Shadow>,
}

impl From<&Shadow> for ShadowState {
    fn from(shadow: &Shadow) -> Self {
        Self { shadow: Some(shadow.clone()) }
    }
}

impl ShadowState {
    pub fn dimension(&self) -> usize {
        self.shadow.as_ref().map(|shadow| shadow.dimension()).unwrap_or_default()
    }
}
