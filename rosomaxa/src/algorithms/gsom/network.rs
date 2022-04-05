#[cfg(test)]
#[path = "../../../tests/unit/algorithms/gsom/network_test.rs"]
mod network_test;

use super::*;
use crate::utils::{parallel_into_collect, Noise, Random};
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
    nodes: HashMap<Coordinate, NodeLink<I, S>>,
    storage_factory: F,
    time: usize,
    rebalance_memory: usize,
    noise: Noise,
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
    /// A random used to generate a noise applied internally to errors and weights.
    pub random: Arc<dyn Random + Send + Sync>,
}

impl<I, S, F> Network<I, S, F>
where
    I: Input,
    S: Storage<Item = I>,
    F: StorageFactory<I, S>,
{
    /// Creates a new instance of `Network`.
    pub fn new(roots: [I; 4], config: NetworkConfig, storage_factory: F) -> Self {
        let dimension = roots[0].weights().len();

        assert!(roots.iter().all(|r| r.weights().len() == dimension));
        assert!(config.distribution_factor > 0. && config.distribution_factor < 1.);

        let growing_threshold = -1. * dimension as f64 * config.spread_factor.log2();
        let initial_error = if config.has_initial_error { growing_threshold } else { 0. };
        let noise = Noise::new(1., (0.95, 1.05), config.random);

        Self {
            dimension,
            growing_threshold,
            distribution_factor: config.distribution_factor,
            learning_rate: config.learning_rate,
            nodes: Self::create_initial_nodes(roots, initial_error, config.rebalance_memory, &noise, &storage_factory),
            storage_factory,
            time: 0,
            rebalance_memory: config.rebalance_memory,
            noise,
        }
    }

    /// Stores input into the network.
    pub fn store(&mut self, input: I, time: usize) {
        debug_assert!(input.weights().len() == self.dimension);
        self.time = time;
        self.train(input, true)
    }

    /// Stores multiple inputs into the network.
    pub fn store_batch<T: Sized + Send + Sync>(&mut self, item_data: Vec<T>, time: usize, map_func: fn(T) -> I) {
        self.time = time;
        self.train_batch(item_data, true, map_func);
    }

    /// Retrains the whole network.
    pub fn retrain(&mut self, rebalance_count: usize, node_filter: &(dyn Fn(&NodeLink<I, S>) -> bool)) {
        // NOTE compact before rebalancing to reduce network size to be rebalanced
        self.compact(node_filter);
        self.rebalance(rebalance_count);
        self.compact(node_filter);
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
    fn train_batch<T: Send + Sync>(&mut self, item_data: Vec<T>, is_new_input: bool, map_func: fn(T) -> I) {
        let nodes_data = parallel_into_collect(item_data, |item| {
            let input = map_func(item);
            let bmu = self.find_bmu(&input);
            let error = bmu.read().unwrap().distance(input.weights());
            (bmu, error, input)
        });

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
        let radius = 2;

        let (exceeds_ae, is_boundary) = {
            let mut node = node.write().unwrap();
            node.error += error;

            // NOTE update usage statistics only for a new input
            if is_new_input {
                node.new_hit(self.time);
            }

            (node.error > self.growing_threshold, node.is_boundary(self))
        };

        match (exceeds_ae, is_boundary) {
            // error distribution
            (true, false) => {
                let mut node = node.write().unwrap();
                node.error = 0.5 * self.growing_threshold;

                node.neighbours(self, radius).for_each(|(n, (x, y))| {
                    if let Some(n) = n {
                        let mut node = n.write().unwrap();
                        let distribution_factor = self.distribution_factor / (x.abs() + y.abs()) as f64;
                        node.error += self.noise.generate(distribution_factor * node.error);
                    }
                });
            }
            // weight distribution
            (true, true) => {
                let node = node.read().unwrap();
                let node_coord = node.coordinate.clone();
                let weights = node.weights.clone();

                // NOTE insert new nodes only in main directions
                #[allow(clippy::needless_collect)]
                let offsets = node
                    .neighbours(self, 1)
                    .filter(|(_, (x, y))| x.abs() + y.abs() < 2)
                    .filter_map(|(node, offset)| if node.is_none() { Some(offset) } else { None })
                    .collect::<Vec<_>>();

                offsets.into_iter().for_each(|(x, y)| {
                    self.insert(Coordinate(node_coord.0 + x, node_coord.1 + y), weights.as_slice());
                });
            }
            _ => {}
        }

        // weight adjustments
        let mut node = node.write().unwrap();
        let learning_rate = self.learning_rate * (1. - 3.8 / (self.nodes.len() as f64));

        node.adjust(input.weights(), learning_rate);
        node.neighbours(self, radius).filter_map(|(n, _)| n).for_each(|n| {
            n.write().unwrap().adjust(input.weights(), learning_rate);
        });
    }

    /// Inserts new neighbors if necessary.
    fn insert(&mut self, coordinate: Coordinate, weights: &[f64]) {
        let new_node = Arc::new(RwLock::new(Node::new(
            coordinate.clone(),
            weights.iter().map(|&value| self.noise.generate(value)).collect::<Vec<_>>(),
            0.,
            self.rebalance_memory,
            self.storage_factory.eval(),
        )));
        self.nodes.insert(coordinate, new_node);
    }

    /// Rebalances network.
    fn rebalance(&mut self, rebalance_count: usize) {
        let mut data = Vec::with_capacity(self.nodes.len());
        (0..rebalance_count).for_each(|_| {
            data.clear();
            data.extend(self.nodes.iter_mut().flat_map(|(_, node)| node.write().unwrap().storage.drain(0..)));

            data.shuffle(&mut rand::thread_rng());

            data.drain(0..).for_each(|input| {
                self.train(input, false);
            });
        });
    }

    fn compact(&mut self, node_filter: &(dyn Fn(&NodeLink<I, S>) -> bool)) {
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

    /// Creates nodes for initial topology.
    fn create_initial_nodes(
        roots: [I; 4],
        initial_error: f64,
        rebalance_memory: usize,
        noise: &Noise,
        storage_factory: &F,
    ) -> HashMap<Coordinate, NodeLink<I, S>> {
        let create_node_link = |coordinate: Coordinate, input: I| {
            let weights = input.weights().iter().map(|&value| noise.generate(value)).collect::<Vec<_>>();
            let mut node =
                Node::<I, S>::new(coordinate, weights, initial_error, rebalance_memory, storage_factory.eval());
            node.storage.add(input);
            Arc::new(RwLock::new(node))
        };

        let [n00, n01, n11, n10] = roots;

        let n00 = create_node_link(Coordinate(0, 0), n00);
        let n01 = create_node_link(Coordinate(0, 1), n01);
        let n11 = create_node_link(Coordinate(1, 1), n11);
        let n10 = create_node_link(Coordinate(1, 0), n10);

        [(Coordinate(0, 0), n00), (Coordinate(0, 1), n01), (Coordinate(1, 1), n11), (Coordinate(1, 0), n10)]
            .iter()
            .cloned()
            .collect()
    }
}
