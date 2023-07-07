#[cfg(test)]
#[path = "../../../tests/unit/algorithms/gsom/network_test.rs"]
mod network_test;

use super::*;
use crate::algorithms::math::get_mean_iter;
use crate::utils::*;
use rand::prelude::SliceRandom;
use rustc_hash::FxHasher;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::iter::once;
use std::sync::Arc;

type NodeHashMap<I, S> = HashMap<Coordinate, Node<I, S>, BuildHasherDefault<FxHasher>>;

/// A customized Growing Self Organizing Map designed to store and retrieve trained input.
pub struct Network<I, S, F>
where
    I: Input,
    S: Storage<Item = I>,
    F: StorageFactory<I, S>,
{
    /// Data dimension.
    dimension: usize,
    /// Growth threshold.
    growing_threshold: f64,
    /// The factor of distribution (FD), used in error distribution stage, 0 < FD < 1
    distribution_factor: f64,
    learning_rate: f64,
    time: usize,
    rebalance_memory: usize,
    min_max_weights: MinMaxWeights,
    nodes: NodeHashMap<I, S>,
    storage_factory: F,
}

/// GSOM network configuration.
pub struct NetworkConfig {
    /// A spread factor.
    pub spread_factor: f64,
    /// The factor of distribution (FD), used in error distribution stage, 0 < FD < 1
    pub distribution_factor: f64,
    /// Initial learning rate.
    pub learning_rate: f64,
    /// A rebalance memory.
    pub rebalance_memory: usize,
    /// If set to true, initial nodes have error set to the value equal to growing threshold.
    pub has_initial_error: bool,
}

/// Specifies min max weights type.
type MinMaxWeights = (Vec<f64>, Vec<f64>);

impl<I, S, F> Network<I, S, F>
where
    I: Input,
    S: Storage<Item = I>,
    F: StorageFactory<I, S>,
{
    /// Creates a new instance of `Network`.
    pub fn new(
        roots: [I; 4],
        config: NetworkConfig,
        random: Arc<dyn Random + Send + Sync>,
        storage_factory: F,
    ) -> Self {
        let dimension = roots[0].weights().len();

        assert!(roots.iter().all(|r| r.weights().len() == dimension));
        assert!(config.distribution_factor > 0. && config.distribution_factor < 1.);
        assert!(config.spread_factor > 0. && config.spread_factor < 1.);

        let growing_threshold = -1. * dimension as f64 * config.spread_factor.log2();
        let initial_error = if config.has_initial_error { growing_threshold } else { 0. };
        let noise = Noise::new_with_ratio(1., (0.75, 1.25), random);

        let (nodes, min_max_weights) =
            Self::create_initial_nodes(roots, initial_error, config.rebalance_memory, &noise, &storage_factory);

        Self {
            dimension,
            growing_threshold,
            distribution_factor: config.distribution_factor,
            learning_rate: config.learning_rate,
            time: 0,
            rebalance_memory: config.rebalance_memory,
            min_max_weights,
            nodes,
            storage_factory,
        }
    }

    /// Sets a new learning rate.
    pub fn set_learning_rate(&mut self, learning_rate: f64) {
        self.learning_rate = learning_rate;
    }

    /// Gets current learning rate.
    pub fn get_learning_rate(&self) -> f64 {
        self.learning_rate
    }

    /// Stores input into the network.
    pub fn store(&mut self, input: I, time: usize) {
        debug_assert!(input.weights().len() == self.dimension);
        self.time = time;
        self.train(input, true)
    }

    /// Stores multiple inputs into the network.
    pub fn store_batch<T: Send + Sync>(&mut self, item_data: Vec<T>, time: usize, map_func: fn(T) -> I) {
        self.time = time;
        let nodes_data = parallel_into_collect(item_data, |item| {
            let input = map_func(item);
            let bmu = self.find_bmu(&input);
            let error = bmu.distance(input.weights());
            (bmu.coordinate, error, input)
        });
        self.train_batch(nodes_data, true);
    }

    /// Performs smoothing phase.
    pub fn smooth(&mut self, rebalance_count: usize) {
        (0..rebalance_count).for_each(|_| {
            let mut data = self.nodes.iter_mut().flat_map(|(_, node)| node.storage.drain(0..)).collect::<Vec<_>>();
            data.shuffle(&mut rand::thread_rng());

            self.train_on_data(data, false);

            self.nodes.iter_mut().for_each(|(_, node)| {
                node.error = 0.;
            })
        });
    }

    /// Compacts network. `node_filter` should return false for nodes to be removed.
    pub fn compact(&mut self) {
        contract_graph(self, (3, 4));
    }

    /// Finds node by its coordinate.
    pub fn find(&self, coord: &Coordinate) -> Option<&Node<I, S>> {
        self.nodes.get(coord)
    }

    /// Returns node coordinates in arbitrary order.
    pub fn get_coordinates(&'_ self) -> impl Iterator<Item = Coordinate> + '_ {
        self.nodes.keys().cloned()
    }

    /// Return nodes in arbitrary order.
    pub fn get_nodes(&self) -> impl Iterator<Item = &Node<I, S>> + '_ {
        self.nodes.values()
    }

    /// Iterates over coordinates and their nodes.
    pub fn iter(&self) -> impl Iterator<Item = (&Coordinate, &Node<I, S>)> {
        self.nodes.iter()
    }

    /// Returns a total amount of nodes.
    pub fn size(&self) -> usize {
        self.nodes.len()
    }

    /// Returns current time.
    pub fn get_current_time(&self) -> usize {
        self.time
    }

    /// Calculates mean distance of nodes with individuals.
    pub fn mean_distance(&self) -> f64 {
        get_mean_iter(self.nodes.iter().filter_map(|(_, node)| node.node_distance()))
    }

    /// Calculates mean squared error of the whole network.
    pub fn mse(&self) -> f64 {
        let n = if self.nodes.is_empty() { 1 } else { self.nodes.len() } as f64;

        self.nodes.iter().fold(0., |acc, (_, node)| acc + node.mse()) / n
    }

    /// Returns max unified distance of the network.
    pub fn max_unified_distance(&self) -> f64 {
        self.get_nodes().map(|node| node.unified_distance(self, 1)).max_by(compare_floats_refs).unwrap_or(0.)
    }

    /// Trains network on an input.
    fn train(&mut self, input: I, is_new_input: bool) {
        debug_assert!(input.weights().len() == self.dimension);

        let (bmu_coord, error) = {
            let bmu = self.find_bmu(&input);
            let error = bmu.distance(input.weights());
            (bmu.coordinate, error)
        };

        self.update(&bmu_coord, &input, error, is_new_input);
        self.nodes.get_mut(&bmu_coord).unwrap().storage.add(input);
    }

    /// Trains network on inputs.
    fn train_batch(&mut self, nodes_data: Vec<(Coordinate, f64, I)>, is_new_input: bool) {
        nodes_data.into_iter().for_each(|(bmu_coord, error, input)| {
            self.update(&bmu_coord, &input, error, is_new_input);
            self.nodes.get_mut(&bmu_coord).unwrap().storage.add(input);
        });
    }

    /// Trains network on given input data.
    pub(super) fn train_on_data(&mut self, data: Vec<I>, is_new_input: bool) {
        let nodes_data = parallel_into_collect(data, |input| {
            let bmu = self.find_bmu(&input);
            let error = bmu.distance(input.weights());
            (bmu.coordinate, error, input)
        });

        self.train_batch(nodes_data, is_new_input);
    }

    /// Finds the best matching unit within the map for the given input.
    fn find_bmu(&self, input: &I) -> &Node<I, S> {
        self.nodes
            .values()
            .map(|node| (node, node.distance(input.weights())))
            .min_by(|(_, x), (_, y)| x.partial_cmp(y).unwrap_or(Ordering::Less))
            .map(|(node, _)| node)
            .expect("no nodes")
    }

    /// Updates network according to the error.
    fn update(&mut self, coord: &Coordinate, input: &I, error: f64, is_new_input: bool) {
        let radius = if is_new_input { 2 } else { 3 };

        let (exceeds_ae, can_grow) = {
            let node = self.nodes.get_mut(coord).expect("invalid coordinate");
            node.error += error;

            // NOTE update usage statistics only for a new input
            if is_new_input {
                node.new_hit(self.time);
            }

            let node = self.nodes.get(coord).unwrap();
            (
                matches!(compare_floats(node.error, self.growing_threshold), Ordering::Equal | Ordering::Greater),
                node.is_boundary(self) && is_new_input,
            )
        };

        match (exceeds_ae, can_grow) {
            (true, false) => self.distribute_error(coord, radius),
            (true, true) => {
                self.grow_nodes(coord).into_iter().for_each(|(coord, weights)| {
                    self.insert(coord, weights.as_slice());
                    self.adjust_weights(&coord, input.weights(), radius, is_new_input);
                });
            }
            _ => self.adjust_weights(coord, input.weights(), radius, is_new_input),
        }
    }

    fn distribute_error(&mut self, coord: &Coordinate, radius: usize) {
        let nodes = once((*coord, None))
            .chain(
                self.nodes
                    .get(coord)
                    .unwrap()
                    .neighbours(self, radius)
                    .filter_map(|(coord, offset)| coord.map(|coord| (coord, offset)))
                    .map(|(coord, (x, y))| {
                        let distribution_factor = self.distribution_factor / (x.abs() + y.abs()) as f64;
                        (coord, Some(distribution_factor))
                    }),
            )
            .collect::<Vec<_>>();

        nodes.into_iter().for_each(|(coord, distribution_factor)| {
            let node = self.nodes.get_mut(&coord).unwrap();
            if let Some(distribution_factor) = distribution_factor {
                node.error += distribution_factor * node.error
            } else {
                node.error = 0.5 * self.growing_threshold
            }
        });
    }

    fn grow_nodes(&self, coord: &Coordinate) -> Vec<(Coordinate, Vec<f64>)> {
        let node = self.nodes.get(coord).unwrap();
        let coord = node.coordinate;
        let weights = node.weights.clone();

        let get_coord = |offset_x: i32, offset_y: i32| Coordinate(coord.0 + offset_x, coord.1 + offset_y);
        let get_node = |offset_x: i32, offset_y: i32| self.nodes.get(&get_coord(offset_x, offset_y));

        // NOTE insert new nodes only in main directions
        node.neighbours(self, 1)
            .filter(|(_, (x, y))| x.abs() + y.abs() < 2)
            .filter_map(|(coord, offset)| if coord.is_none() { Some(offset) } else { None })
            .map(|(n_x, n_y)| {
                let coord = get_coord(n_x, n_y);
                let offset_abs = (n_x.abs(), n_y.abs());

                let weights = match offset_abs {
                    (1, 0) => get_node(n_x * 2, 0),
                    (0, 1) => get_node(0, n_y * 2),
                    _ => unreachable!(),
                }
                .map(|w2| {
                    // case b
                    weights.as_slice().iter().zip(w2.weights.iter()).map(|(&w1, &w2)| (w1 + w2) / 2.).collect()
                })
                .unwrap_or_else(|| {
                    // case a
                    match offset_abs {
                        (1, 0) => get_node(-n_x, 0),
                        (0, 1) => get_node(0, -n_y),
                        _ => unreachable!(),
                    }
                    // case c
                    .or_else(|| match offset_abs {
                        (1, 0) => get_node(0, 1).or_else(|| get_node(0, -1)),
                        (0, 1) => get_node(1, 0).or_else(|| get_node(-1, 0)),
                        _ => unreachable!(),
                    })
                    .map(|w2| {
                        // cases a & c
                        weights
                            .as_slice()
                            .iter()
                            .zip(w2.weights.iter())
                            .map(|(&w1, &w2)| if w2 > w1 { w1 - (w2 - w1) } else { w1 + (w1 - w2) })
                            .collect()
                    })
                    // case d
                    .unwrap_or_else(|| {
                        self.min_max_weights
                            .0
                            .iter()
                            .zip(self.min_max_weights.1.iter())
                            .map(|(min, max)| (min + max) / 2.)
                            .collect()
                    })
                });

                (coord, weights)
            })
            .collect()
    }

    fn adjust_weights(&mut self, coord: &Coordinate, weights: &[f64], radius: usize, is_new_input: bool) {
        let node = self.nodes.get(coord).expect("invalid coordinate");
        let learning_rate = self.learning_rate * (1. - 3.8 / (self.nodes.len() as f64));
        let learning_rate = if is_new_input { learning_rate } else { 0.25 * learning_rate };

        let nodes = once((*coord, weights, learning_rate))
            .chain(node.neighbours(self, radius).filter_map(|(coord, offset)| coord.map(|coord| (coord, offset))).map(
                |(coord, offset)| {
                    let distance = offset.0.abs() + offset.1.abs();
                    let learning_rate = learning_rate / distance as f64;
                    (coord, weights, learning_rate)
                },
            ))
            .collect::<Vec<_>>();

        nodes.into_iter().for_each(|(coord, weights, learning_rate)| {
            self.nodes.get_mut(&coord).unwrap().adjust(weights, learning_rate);
        })
    }

    /// Gets a mutable reference for node with given coordinate.
    pub(super) fn get_mut(&mut self, coord: &Coordinate) -> Option<&mut Node<I, S>> {
        self.nodes.get_mut(coord)
    }

    /// Inserts new neighbors if necessary.
    pub(super) fn insert(&mut self, coord: Coordinate, weights: &[f64]) {
        update_min_max(&mut self.min_max_weights, weights);
        self.nodes.insert(coord, self.create_node(coord, weights, 0.));
    }

    /// Removes node with given coordinate.
    pub(super) fn remove(&mut self, coord: &Coordinate) {
        self.nodes.remove(coord);
    }

    /// Remaps internal lattice after potential changes in coordinate schema.
    pub(super) fn remap(&mut self, node_modifier: &(dyn Fn(Coordinate, Node<I, S>) -> Node<I, S>)) {
        let nodes = self.nodes.drain().map(|(coord, node)| node_modifier(coord, node)).collect::<Vec<_>>();
        self.nodes.extend(nodes.into_iter().map(|node| (node.coordinate, node)));
    }

    /// Returns data (weights) dimension.
    pub(super) fn dimension(&self) -> usize {
        self.dimension
    }

    /// Creates a new node for given data.
    fn create_node(&self, coord: Coordinate, weights: &[f64], error: f64) -> Node<I, S> {
        Node::new(coord, weights, error, self.rebalance_memory, self.storage_factory.eval())
    }

    /// Creates nodes for initial topology.
    fn create_initial_nodes(
        roots: [I; 4],
        initial_error: f64,
        rebalance_memory: usize,
        noise: &Noise,
        storage_factory: &F,
    ) -> (NodeHashMap<I, S>, MinMaxWeights) {
        let create_node = |coord: Coordinate, input: I| {
            let weights = input.weights().iter().map(|&value| noise.generate(value)).collect::<Vec<_>>();
            let mut node =
                Node::<I, S>::new(coord, weights.as_slice(), initial_error, rebalance_memory, storage_factory.eval());
            node.storage.add(input);

            node
        };

        let dimension = roots[0].weights().len();
        let [n00, n01, n11, n10] = roots;

        let n00 = create_node(Coordinate(0, 0), n00);
        let n01 = create_node(Coordinate(0, 1), n01);
        let n11 = create_node(Coordinate(1, 1), n11);
        let n10 = create_node(Coordinate(1, 0), n10);

        let min_max_weights = [&n00, &n01, &n11, &n10].into_iter().fold(
            (vec![f64::MAX; dimension], vec![f64::MIN; dimension]),
            |mut min_max_weights, node| {
                update_min_max(&mut min_max_weights, node.weights.as_slice());

                min_max_weights
            },
        );

        let nodes = [n00, n01, n11, n10].into_iter().map(|node| (node.coordinate, node)).collect::<HashMap<_, _, _>>();

        (nodes, min_max_weights)
    }
}

fn update_min_max(min_max_weights: &mut (Vec<f64>, Vec<f64>), weights: &[f64]) {
    min_max_weights.0.iter_mut().zip(weights.iter()).for_each(|(curr, v)| *curr = curr.min(*v));
    min_max_weights.1.iter_mut().zip(weights.iter()).for_each(|(curr, v)| *curr = curr.max(*v));
}
