use super::*;
use hashbrown::HashMap;
use rand::prelude::SliceRandom;
use std::cmp::Ordering;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

/// A customized Growing Self Organizing Map.
pub struct Network<I: Input, S: Storage<Item = I>> {
    /// Data dimension.
    dimension: usize,
    /// Growth threshold.
    growing_threshold: f64,
    /// The reduction factor of learning rate.
    reduction_factor: f64,
    /// The factor of distribution (FD), used in error distribution stage, 0 < FD < 1
    distribution_factor: f64,
    /// Initial learning rate.
    learning_rate: f64,
    /// All nodes in the network.
    nodes: HashMap<Coordinate, NodeLink<I, S>>,
    /// Creates input storage for new nodes.
    storage_factory: Box<dyn Fn() -> S + Send + Sync>,
    /// A current time which is used to track node update statistics.
    time: usize,
    /// A hit memory of the node.
    hit_memory: usize,
}

impl<I: Input, S: Storage<Item = I>> Network<I, S> {
    /// Creates a new instance of `Network`.
    pub fn new(
        roots: [I; 4],
        spread_factor: f64,
        reduction_factor: f64,
        distribution_factor: f64,
        learning_rate: f64,
        hit_memory: usize,
        storage_factory: Box<dyn Fn() -> S + Send + Sync>,
    ) -> Self {
        let dimension = roots[0].weights().len();

        assert!(roots.iter().all(|r| r.weights().len() == dimension));
        assert!(reduction_factor > 0. && reduction_factor < 1.);
        assert!(distribution_factor > 0. && distribution_factor < 1.);

        Self {
            dimension,
            growing_threshold: -1. * dimension as f64 * spread_factor.log2(),
            reduction_factor,
            distribution_factor,
            learning_rate,
            nodes: Self::create_initial_nodes(roots, hit_memory, &storage_factory),
            storage_factory,
            time: 0,
            hit_memory,
        }
    }

    /// Adds to the network a new input.
    pub fn store(&mut self, input: I, time: usize) {
        debug_assert!(input.weights().len() == self.dimension);

        self.time = time;

        self.train(input, true)
    }

    /// Optimizes network by rebalancing and compaction of the nodes.
    pub fn optimize(&mut self, rebalance_count: usize, compact_rule: &(dyn Fn(&NodeLink<I, S>) -> bool)) {
        self.rebalance(rebalance_count);
        self.compact(compact_rule);
    }

    /// Returns node coordinates in arbitrary order.
    pub fn get_coordinates<'a>(&'a self) -> impl Iterator<Item = Coordinate> + 'a {
        self.nodes.keys().cloned()
    }

    /// Return nodes in arbitrary order.
    pub fn get_nodes<'a>(&'a self) -> impl Iterator<Item = &NodeLink<I, S>> + 'a {
        self.nodes.values()
    }

    /// Checks whether hit time is considered as last.
    pub fn is_last_hit(&self, hit_time: usize) -> bool {
        hit_time as i32 >= self.time as i32 - self.hit_memory as i32
    }

    /// Returns a total amount of nodes.
    pub fn size(&self) -> usize {
        self.nodes.len()
    }

    /// Trains network on an input.
    fn train(&mut self, input: I, is_new_input: bool) {
        debug_assert!(input.weights().len() == self.dimension);

        let bmu = self.find_bmu(&input);
        let error = bmu.read().unwrap().distance(input.weights());

        self.update(&bmu, &input, error, is_new_input);

        bmu.write().unwrap().storage.add(input);
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
        let (exceeds_ae, is_boundary) = {
            let mut node = node.write().unwrap();
            node.error += error;

            // NOTE update usage statistics only for a new input
            if is_new_input {
                node.new_hit(self.time);
            }

            (node.error > self.growing_threshold, node.topology.is_boundary())
        };

        match (exceeds_ae, is_boundary) {
            // error distribution
            (true, false) => {
                let distribute_error = |node: Option<&NodeLink<I, S>>| {
                    let mut node = node.unwrap().write().unwrap();
                    node.error += self.distribution_factor * node.error;
                };

                let mut node = node.write().unwrap();

                node.error = 0.5 * self.growing_threshold;

                distribute_error(node.topology.left.as_ref());
                distribute_error(node.topology.right.as_ref());
                distribute_error(node.topology.up.as_ref());
                distribute_error(node.topology.down.as_ref());
            }
            // weight distribution
            (true, true) => {
                // NOTE clone to fight with borrow checker
                let coordinate = node.read().unwrap().coordinate.clone();
                let weights = node.read().unwrap().weights.clone();
                let topology = node.read().unwrap().topology.clone();

                let mut distribute_weight = |offset: (i32, i32), link: Option<&NodeLink<I, S>>| {
                    if link.is_none() {
                        let coordinate = Coordinate(coordinate.0 + offset.0, coordinate.1 + offset.1);
                        self.insert(coordinate, weights.as_slice());
                    }
                };

                distribute_weight((-1, 0), topology.left.as_ref());
                distribute_weight((1, 0), topology.right.as_ref());
                distribute_weight((0, 1), topology.up.as_ref());
                distribute_weight((0, -1), topology.down.as_ref());
            }
            _ => {}
        }

        // weight adjustments
        let mut node = node.write().unwrap();
        let learning_rate = self.learning_rate * self.reduction_factor * (1. - 3.8 / (self.nodes.len() as f64));

        node.adjust(input.weights(), learning_rate);
        (node.topology.neighbours().map(|n| n.write().unwrap())).for_each(|mut neighbor| {
            neighbor.adjust(input.weights(), learning_rate);
        });
    }

    /// Inserts new neighbors if necessary.
    fn insert(&mut self, coordinate: Coordinate, weights: &[f64]) {
        let new_node = Arc::new(RwLock::new(Node::new(
            coordinate.clone(),
            weights,
            self.hit_memory,
            self.storage_factory.deref()(),
        )));
        {
            let mut new_node_mut = new_node.write().unwrap();
            let (new_x, new_y) = (coordinate.0, coordinate.1);

            if let Some(node) = self.nodes.get(&Coordinate(new_x - 1, new_y)) {
                new_node_mut.topology.left = Some(node.clone());
                node.write().unwrap().topology.right = Some(new_node.clone());
            }

            if let Some(node) = self.nodes.get(&Coordinate(new_x + 1, new_y)) {
                new_node_mut.topology.right = Some(node.clone());
                node.write().unwrap().topology.left = Some(new_node.clone());
            }

            if let Some(node) = self.nodes.get(&Coordinate(new_x, new_y - 1)) {
                new_node_mut.topology.down = Some(node.clone());
                node.write().unwrap().topology.up = Some(new_node.clone());
            }

            if let Some(node) = self.nodes.get(&Coordinate(new_x, new_y + 1)) {
                new_node_mut.topology.up = Some(node.clone());
                node.write().unwrap().topology.down = Some(new_node.clone());
            }
        }

        self.nodes.insert(coordinate, new_node);
    }

    /// Rebalances network.
    fn rebalance(&mut self, rebalance_count: usize) {
        let mut data = Vec::with_capacity(self.nodes.len());
        (0..rebalance_count).for_each(|_| {
            data.clear();
            data.extend(self.nodes.iter_mut().flat_map(|(_, node)| node.write().unwrap().storage.drain()));

            data.shuffle(&mut rand::thread_rng());

            data.drain(0..).for_each(|input| {
                self.train(input, false);
            });
        });
    }

    /// Compacts network.
    fn compact(&mut self, rule: &(dyn Fn(&NodeLink<I, S>) -> bool)) {
        let mut remove = vec![];
        self.nodes.iter_mut().filter(|(_, node)| rule.deref()(node)).for_each(|(coordinate, node)| {
            let topology = &mut node.write().unwrap().topology;
            topology.left.iter_mut().for_each(|link| link.write().unwrap().topology.right = None);
            topology.right.iter_mut().for_each(|link| link.write().unwrap().topology.left = None);
            topology.up.iter_mut().for_each(|link| link.write().unwrap().topology.down = None);
            topology.down.iter_mut().for_each(|link| link.write().unwrap().topology.up = None);

            remove.push(coordinate.clone());
        });

        remove.iter().for_each(|coordinate| {
            self.nodes.remove(coordinate);
        })
    }

    /// Creates nodes for initial topology.
    fn create_initial_nodes(
        roots: [I; 4],
        hit_memory: usize,
        storage_factory: &Box<dyn Fn() -> S + Send + Sync>,
    ) -> HashMap<Coordinate, NodeLink<I, S>> {
        let create_node_link = |coordinate: Coordinate, input: I| {
            let mut node = Node::<I, S>::new(coordinate, input.weights(), hit_memory, storage_factory.deref()());
            node.storage.add(input);
            Arc::new(RwLock::new(node))
        };

        let [n00, n01, n11, n10] = roots;

        let n00 = create_node_link(Coordinate(0, 0), n00);
        let n01 = create_node_link(Coordinate(0, 1), n01);
        let n11 = create_node_link(Coordinate(1, 1), n11);
        let n10 = create_node_link(Coordinate(1, 0), n10);

        n00.write().unwrap().topology.right = Some(n10.clone());
        n00.write().unwrap().topology.up = Some(n01.clone());

        n01.write().unwrap().topology.right = Some(n11.clone());
        n01.write().unwrap().topology.down = Some(n00.clone());

        n10.write().unwrap().topology.up = Some(n11.clone());
        n10.write().unwrap().topology.left = Some(n00.clone());

        n11.write().unwrap().topology.left = Some(n01.clone());
        n11.write().unwrap().topology.down = Some(n10.clone());

        [(Coordinate(0, 0), n00), (Coordinate(0, 1), n01), (Coordinate(1, 1), n11), (Coordinate(1, 0), n10)]
            .iter()
            .cloned()
            .collect()
    }
}
