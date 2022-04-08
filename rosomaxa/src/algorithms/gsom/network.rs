#[cfg(test)]
#[path = "../../../tests/unit/algorithms/gsom/network_test.rs"]
mod network_test;

use super::*;
use crate::utils::{compare_floats, parallel_into_collect, Noise, Random};
use hashbrown::HashMap;
use rand::prelude::SliceRandom;
use std::cmp::Ordering;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

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
    nodes: HashMap<Coordinate, NodeLink<I, S>>,
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

        let growing_threshold = -1. * dimension as f64 * config.spread_factor.log2();
        let initial_error = if config.has_initial_error { growing_threshold } else { 0. };
        let noise = Noise::new(1., (0.95, 1.05), random);

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
            let error = bmu.read().unwrap().distance(input.weights());
            (bmu, error, input)
        });
        self.train_batch(nodes_data, true);
    }

    /// Performs smoothing phase.
    pub fn smooth(&mut self, rebalance_count: usize) {
        (0..rebalance_count).for_each(|_| {
            let mut data = self
                .nodes
                .iter_mut()
                .flat_map(|(_, node)| node.write().unwrap().storage.drain(0..))
                .collect::<Vec<_>>();
            data.shuffle(&mut rand::thread_rng());

            let nodes_data = parallel_into_collect(data, |input| {
                let bmu = self.find_bmu(&input);
                let error = bmu.read().unwrap().distance(input.weights());
                (bmu, error, input)
            });

            self.train_batch(nodes_data, false);
        });
    }

    /// Compacts network.
    pub fn compact(&mut self, node_filter: &(dyn Fn(&NodeLink<I, S>) -> bool)) {
        let original = self.nodes.len();
        let mut removed = vec![];
        let mut remove_node = |coordinate: &Coordinate| {
            // NOTE: prevent network to be less than 4 nodes
            if (original - removed.len()) > 4 {
                removed.push(coordinate.clone());
            }
        };

        // remove user defined nodes
        self.nodes
            .iter_mut()
            .filter(|(_, node)| !node_filter.deref()(node))
            .for_each(|(coordinate, _)| remove_node(coordinate));

        removed.iter().for_each(|coordinate| {
            self.nodes.remove(coordinate);
        });
    }

    /// Finds node by its coordinate.
    pub fn find(&self, coordinate: &Coordinate) -> Option<&NodeLink<I, S>> {
        self.nodes.get(coordinate)
    }

    /// Returns node coordinates in arbitrary order.
    pub fn get_coordinates(&'_ self) -> impl Iterator<Item = Coordinate> + '_ {
        self.nodes.keys().cloned()
    }

    /// Return nodes in arbitrary order.
    pub fn get_nodes<'a>(&'a self) -> impl Iterator<Item = &NodeLink<I, S>> + 'a {
        self.nodes.values()
    }

    /// Iterates over coordinates and their nodes.
    pub fn iter(&self) -> impl Iterator<Item = (&Coordinate, &NodeLink<I, S>)> {
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

    /// Trains network on an input.
    fn train(&mut self, input: I, is_new_input: bool) {
        debug_assert!(input.weights().len() == self.dimension);

        let bmu = self.find_bmu(&input);
        let error = bmu.read().unwrap().distance(input.weights());

        self.update(&bmu, &input, error, is_new_input);

        bmu.write().unwrap().storage.add(input);
    }

    /// Trains network on inputs.
    fn train_batch(&mut self, nodes_data: Vec<(NodeLink<I, S>, f64, I)>, is_new_input: bool) {
        nodes_data.into_iter().for_each(|(bmu, error, input)| {
            self.update(&bmu, &input, error, is_new_input);
            bmu.write().unwrap().storage.add(input);
        });
    }

    /// Finds the best matching unit within the map for the given input.
    fn find_bmu(&self, input: &I) -> NodeLink<I, S> {
        self.nodes
            .iter()
            .map(|(_, node)| (node.clone(), node.read().unwrap().distance(input.weights())))
            .min_by(|(_, x), (_, y)| x.partial_cmp(y).unwrap_or(Ordering::Less))
            .map(|(node, _)| node)
            .expect("no nodes")
    }

    /// Updates network according to the error.
    fn update(&mut self, node: &NodeLink<I, S>, input: &I, error: f64, is_new_input: bool) {
        let radius = if is_new_input { 2 } else { 1 };

        let (exceeds_ae, can_grow) = {
            let mut node = node.write().unwrap();
            node.error += error;

            // NOTE update usage statistics only for a new input
            if is_new_input {
                node.new_hit(self.time);
            }

            (
                matches!(compare_floats(node.error, self.growing_threshold), Ordering::Equal | Ordering::Greater),
                node.is_boundary(self) && is_new_input,
            )
        };

        match (exceeds_ae, can_grow) {
            (true, false) => self.distribute_error(node, radius),
            (true, true) => {
                self.grow_nodes(node).into_iter().for_each(|(coordinate, weights)| {
                    self.insert(coordinate.clone(), weights.as_slice());
                    let new_node = self.nodes.get(&coordinate).unwrap();
                    self.adjust_weights(new_node, input.weights(), radius, is_new_input);
                });
            }
            _ => self.adjust_weights(node, input.weights(), radius, is_new_input),
        }
    }

    fn distribute_error(&self, node: &NodeLink<I, S>, radius: usize) {
        let mut node = node.write().unwrap();
        node.error = 0.5 * self.growing_threshold;

        node.neighbours(self, radius).for_each(|(n, (x, y))| {
            if let Some(n) = n {
                let mut node = n.write().unwrap();
                let distribution_factor = self.distribution_factor / (x.abs() + y.abs()) as f64;
                node.error += distribution_factor * node.error;
            }
        });
    }

    #[allow(clippy::needless_collect)]
    fn grow_nodes(&self, node: &NodeLink<I, S>) -> Vec<(Coordinate, Vec<f64>)> {
        let node = node.read().unwrap();
        let coord = node.coordinate.clone();
        let weights = node.weights.clone();

        let get_coord = |offset_x: i32, offset_y: i32| Coordinate(coord.0 + offset_x, coord.1 + offset_y);
        let get_node = |offset_x: i32, offset_y: i32| self.nodes.get(&get_coord(offset_x, offset_y));

        // NOTE insert new nodes only in main directions
        let offsets = node
            .neighbours(self, 1)
            .filter(|(_, (x, y))| x.abs() + y.abs() < 2)
            .filter_map(|(node, offset)| if node.is_none() { Some(offset) } else { None })
            .collect::<Vec<_>>();

        offsets
            .into_iter()
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
                    weights
                        .as_slice()
                        .iter()
                        .zip(w2.read().unwrap().weights.iter())
                        .map(|(&w1, &w2)| (w1 + w2) / 2.)
                        .collect()
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
                            .zip(w2.read().unwrap().weights.iter())
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

    fn adjust_weights(&self, node: &NodeLink<I, S>, weights: &[f64], radius: usize, is_new_input: bool) {
        let mut node = node.write().unwrap();
        let learning_rate = self.learning_rate * (1. - 3.8 / (self.nodes.len() as f64));
        let learning_rate = if is_new_input { 0.25 * learning_rate } else { learning_rate };

        node.adjust(weights, learning_rate);
        node.neighbours(self, radius).filter_map(|(n, offset)| n.map(|n| (n, offset))).for_each(|(n, offset)| {
            let distance = offset.0.abs() + offset.1.abs();
            let learning_rate = learning_rate / distance as f64;
            n.write().unwrap().adjust(weights, learning_rate);
        });
    }

    /// Inserts new neighbors if necessary.
    fn insert(&mut self, coordinate: Coordinate, weights: &[f64]) {
        update_min_max(&mut self.min_max_weights, weights);
        self.nodes.insert(coordinate.clone(), Arc::new(RwLock::new(self.create_node(coordinate, weights, 0.))));
    }

    /// Creates a new node for given data.
    fn create_node(&self, coordinate: Coordinate, weights: &[f64], error: f64) -> Node<I, S> {
        Node::new(coordinate, weights, error, self.rebalance_memory, self.storage_factory.eval())
    }

    /// Creates nodes for initial topology.
    fn create_initial_nodes(
        roots: [I; 4],
        initial_error: f64,
        rebalance_memory: usize,
        noise: &Noise,
        storage_factory: &F,
    ) -> (HashMap<Coordinate, NodeLink<I, S>>, MinMaxWeights) {
        let create_node_link = |coordinate: Coordinate, input: I| {
            let weights = input.weights().iter().map(|&value| noise.generate(value)).collect::<Vec<_>>();
            let mut node = Node::<I, S>::new(
                coordinate,
                weights.as_slice(),
                initial_error,
                rebalance_memory,
                storage_factory.eval(),
            );
            node.storage.add(input);
            Arc::new(RwLock::new(node))
        };

        let dimension = roots[0].weights().len();
        let [n00, n01, n11, n10] = roots;

        let n00 = create_node_link(Coordinate(0, 0), n00);
        let n01 = create_node_link(Coordinate(0, 1), n01);
        let n11 = create_node_link(Coordinate(1, 1), n11);
        let n10 = create_node_link(Coordinate(1, 0), n10);

        let nodes =
            [(Coordinate(0, 0), n00), (Coordinate(0, 1), n01), (Coordinate(1, 1), n11), (Coordinate(1, 0), n10)]
                .iter()
                .cloned()
                .collect::<HashMap<_, _>>();

        let min_max_weights = nodes.iter().fold(
            (vec![f64::MAX; dimension], vec![f64::MIN; dimension]),
            |mut min_max_weights, (_, node)| {
                let weights = node.read().unwrap().weights.clone();
                update_min_max(&mut min_max_weights, weights.as_slice());

                min_max_weights
            },
        );

        (nodes, min_max_weights)
    }
}

fn update_min_max(min_max_weights: &mut (Vec<f64>, Vec<f64>), weights: &[f64]) {
    min_max_weights.0.iter_mut().zip(weights.iter()).for_each(|(curr, v)| *curr = curr.min(*v));
    min_max_weights.1.iter_mut().zip(weights.iter()).for_each(|(curr, v)| *curr = curr.max(*v));
}
