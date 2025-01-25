#[cfg(test)]
#[path = "../../../tests/unit/algorithms/gsom/network_test.rs"]
mod network_test;

use super::*;
use crate::utils::*;
use rand::prelude::SliceRandom;
use rayon::iter::Either;
use rustc_hash::FxHasher;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::iter::once;
use std::marker::PhantomData;
use std::sync::Arc;

type NodeHashMap<I, S> = HashMap<Coordinate, Node<I, S>, BuildHasherDefault<FxHasher>>;

/// A customized Growing Self Organizing Map designed to store and retrieve trained input.
pub struct Network<C, I, S, F>
where
    C: Send + Sync,
    I: Input,
    S: Storage<Item = I>,
    F: StorageFactory<C, I, S>,
{
    /// Data dimension.
    dimension: usize,
    /// Growth threshold.
    growing_threshold: Float,
    /// The factor of distribution (FD), used in error distribution stage, 0 < FD < 1
    distribution_factor: Float,
    learning_rate: Float,
    time: usize,
    rebalance_memory: usize,
    min_max_weights: MinMaxWeights,
    nodes: NodeHashMap<I, S>,
    storage_factory: F,
    random: Arc<dyn Random>,
    phantom_data: PhantomData<C>,
}

/// GSOM network configuration.
#[derive(Clone, Debug)]
pub struct NetworkConfig {
    /// A size of a node in the storage.
    pub node_size: usize,
    /// A spread factor.
    pub spread_factor: Float,
    /// The factor of distribution (FD), used in error distribution stage, 0 < FD < 1
    pub distribution_factor: Float,
    /// Initial learning rate.
    pub learning_rate: Float,
    /// A rebalance memory.
    pub rebalance_memory: usize,
    /// If set to true, initial nodes have error set to the value equal to a growing threshold.
    pub has_initial_error: bool,
}

/// Specifies min max weights type.
pub struct MinMaxWeights {
    /// Min weights.
    pub min: Vec<Float>,
    /// Max weights.
    pub max: Vec<Float>,
    /// Indicates if min-max values are reset.
    is_reset: bool,
}

impl MinMaxWeights {
    /// Creates a new instance of [MinMaxWeights].
    fn new(dimension: usize) -> Self {
        Self { min: vec![Float::MAX; dimension], max: vec![Float::MIN; dimension], is_reset: true }
    }

    /// Updates min max weights.
    pub fn update(&mut self, weights: &[Float]) {
        debug_assert!(weights.len() == self.min.len());

        self.min.iter_mut().zip(weights.iter()).for_each(|(curr, v)| *curr = curr.min(*v));
        self.max.iter_mut().zip(weights.iter()).for_each(|(curr, v)| *curr = curr.max(*v));
        self.is_reset = false;
    }

    /// Iterates over min-max values.
    pub fn iter(&self) -> impl Iterator<Item = (Float, Float)> + '_ {
        if self.is_reset {
            // return min=0, max=1 to prevent numerical issues
            Either::Left((0..self.min.len()).map(|_| (0. as Float, 1. as Float)))
        } else {
            Either::Right(self.min.iter().copied().zip(self.max.iter().copied()))
        }
    }

    /// Resets min-max values.
    pub fn reset(&mut self) {
        self.is_reset = true;
        self.min.fill(Float::MAX);
        self.max.fill(Float::MIN);
    }
}

impl<C, I, S, F> Network<C, I, S, F>
where
    C: Send + Sync,
    I: Input,
    S: Storage<Item = I>,
    F: StorageFactory<C, I, S>,
{
    /// Creates a new instance of `Network`.
    pub fn new<SF>(
        context: &C,
        initial_data: Vec<I>,
        config: NetworkConfig,
        random: Arc<dyn Random>,
        storage_factory: SF,
    ) -> GenericResult<Self>
    where
        SF: Fn(usize) -> F,
    {
        assert!(!initial_data.is_empty());
        let dimension = initial_data[0].weights().len();
        let data_size = initial_data.len();
        assert!(initial_data.iter().all(|r| r.weights().len() == dimension));
        assert!(config.distribution_factor > 0. && config.distribution_factor < 1.);
        assert!(config.spread_factor > 0. && config.spread_factor < 1.);

        // create initial nodes
        // note that storage factory creates storage with size up to data_size
        // it should help to prevent data lost until the network is rebalanced
        let (nodes, min_max_weights) = Self::create_initial_nodes(
            context,
            initial_data,
            config.rebalance_memory,
            &storage_factory(data_size),
            // apply small noise to initial weights
            Noise::new_with_ratio(1., (0.99, 1.), random.clone()),
        )?;

        // create a network with more aggressive initial parameters
        let mut network = Self {
            dimension,
            growing_threshold: -1. * dimension as Float * config.spread_factor.log2(),
            distribution_factor: config.distribution_factor,
            learning_rate: config.learning_rate,
            time: 0,
            rebalance_memory: config.rebalance_memory,
            min_max_weights,
            nodes,
            storage_factory: storage_factory(data_size),
            random: random.clone(),
            phantom_data: Default::default(),
        };

        // run training loop to balance the network
        let allow_growth = true;
        let rebalance_count = (data_size / 4).clamp(8, 12);
        network.retrain(context, rebalance_count, allow_growth, |_| ());

        // reset to original parameters and make sure that node storages have the desired size
        network.storage_factory = storage_factory(config.node_size);
        network.time = 0;
        network.nodes.iter_mut().for_each(|(_, node)| {
            node.storage.resize(config.node_size);
        });

        Ok(network)
    }

    /// Sets a new learning rate.
    pub fn set_learning_rate(&mut self, learning_rate: Float) {
        self.learning_rate = learning_rate;
    }

    /// Gets current learning rate.
    pub fn get_learning_rate(&self) -> Float {
        self.learning_rate
    }

    /// Stores input into the network.
    pub fn store(&mut self, context: &C, input: I, time: usize) {
        debug_assert!(input.weights().len() == self.dimension);
        self.time = time;
        self.min_max_weights.update(input.weights());
        self.train(context, input, true)
    }

    /// Stores multiple inputs into the network.
    pub fn store_batch(&mut self, context: &C, data: Vec<I>, time: usize) {
        self.time = time;
        data.iter().for_each(|i| self.min_max_weights.update(i.weights()));

        self.train_on_data(context, data, true);
    }

    /// Performs smoothing phase.
    pub fn smooth<FM>(&mut self, context: &C, rebalance_count: usize, node_fn: FM)
    where
        FM: Fn(&mut I),
    {
        let allow_growth = false;
        self.retrain(context, rebalance_count, allow_growth, node_fn);
    }

    /// Compacts network. `node_filter` should return false for nodes to be removed.
    pub fn compact(&mut self, context: &C) {
        contract_graph(context, self, (3, 4));
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

    /// Calculates mean squared error of the whole network.
    pub fn mse(&self) -> Float {
        let (n, sum) = self
            .nodes
            .iter()
            .filter(|(_, node)| node.storage.size() > 0)
            .fold((0, 0.), |(n, sum), (_, node)| (n + 1, sum + node.mse(self)));

        if n > 0 {
            sum / n as Float
        } else {
            0.
        }
    }

    /// Returns max unified distance of the network.
    pub fn max_unified_distance(&self) -> Float {
        self.get_nodes().map(|node| node.unified_distance(self, 1)).max_by(|a, b| a.total_cmp(b)).unwrap_or_default()
    }

    /// Performs training loop multiple times.
    fn retrain<FM>(&mut self, context: &C, rebalance_count: usize, allow_growth: bool, node_fn: FM)
    where
        FM: Fn(&mut I),
    {
        (0..rebalance_count).for_each(|_| {
            let mut data = self.nodes.iter_mut().flat_map(|(_, node)| node.storage.drain(0..)).collect::<Vec<_>>();
            data.sort_unstable_by(compare_input);
            data.dedup_by(|a, b| compare_input(a, b) == Ordering::Equal);
            data.shuffle(&mut self.random.get_rng());
            data.iter_mut().for_each(&node_fn);

            // update min max weights to reflect the current state
            self.min_max_weights.reset();
            data.iter().for_each(|i| self.min_max_weights.update(i.weights()));
            self.nodes.iter().for_each(|(_, node)| {
                self.min_max_weights.update(node.weights.as_slice());
            });

            self.train_on_data(context, data, allow_growth);

            self.nodes.iter_mut().for_each(|(_, node)| {
                node.error = 0.;
            })
        });
    }

    /// Trains network on an input.
    fn train(&mut self, context: &C, input: I, is_new_input: bool) {
        debug_assert!(input.weights().len() == self.dimension);

        let (bmu_coord, error) = {
            let bmu = self.find_bmu(&input);
            let error = self.distance(&bmu.weights, input.weights());
            (bmu.coordinate, error)
        };

        self.update(context, &bmu_coord, &input, error, is_new_input);
        self.nodes.get_mut(&bmu_coord).unwrap().storage.add(input);
    }

    /// Trains network on inputs.
    fn train_batch(&mut self, context: &C, nodes_data: Vec<(Coordinate, Float, I)>, is_new_input: bool) {
        nodes_data.into_iter().for_each(|(bmu_coord, error, input)| {
            self.update(context, &bmu_coord, &input, error, is_new_input);
            self.nodes.get_mut(&bmu_coord).unwrap().storage.add(input);
        });
    }

    /// Trains network on given input data.
    pub(super) fn train_on_data(&mut self, context: &C, data: Vec<I>, is_new_input: bool) {
        let nodes_data = parallel_into_collect(data, |input| {
            let bmu = self.find_bmu(&input);
            let error = self.distance(&bmu.weights, input.weights());
            (bmu.coordinate, error, input)
        });

        self.train_batch(context, nodes_data, is_new_input);
    }

    /// Finds the best matching unit within the map for the given input.
    fn find_bmu(&self, input: &I) -> &Node<I, S> {
        self.nodes
            .values()
            .map(|node| (node, self.distance(&node.weights, input.weights())))
            .min_by(|(_, x), (_, y)| x.partial_cmp(y).unwrap_or(Ordering::Less))
            .map(|(node, _)| node)
            .expect("no nodes")
    }

    /// Updates network, according to the error.
    fn update(&mut self, context: &C, coord: &Coordinate, input: &I, error: Float, is_new_input: bool) {
        let radius = if is_new_input { 2 } else { 3 };

        let (exceeds_ae, can_grow) = {
            let node = self.nodes.get_mut(coord).expect("invalid coordinate");
            node.error += error;

            // NOTE update usage statistics only for a new input
            if is_new_input {
                node.new_hit(self.time);
            }

            let node = self.nodes.get(coord).unwrap();
            (node.error >= self.growing_threshold, node.is_boundary(self) && is_new_input)
        };

        match (exceeds_ae, can_grow) {
            (true, false) => self.distribute_error(coord, radius),
            (true, true) => {
                self.grow_nodes(coord).into_iter().for_each(|(coord, weights)| {
                    self.insert(context, coord, weights.as_slice());
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
                        let distribution_factor = self.distribution_factor / (x.abs() + y.abs()) as Float;
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

    fn grow_nodes(&self, coord: &Coordinate) -> Vec<(Coordinate, Vec<Float>)> {
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
                    .unwrap_or_else(|| self.min_max_weights.iter().map(|(min, max)| (min + max) / 2.).collect())
                });

                (coord, weights)
            })
            .collect()
    }

    fn adjust_weights(&mut self, coord: &Coordinate, weights: &[Float], radius: usize, is_new_input: bool) {
        let node = self.nodes.get(coord).expect("invalid coordinate");
        let learning_rate = self.learning_rate * (1. - 3.8 / (self.nodes.len() as Float));
        let learning_rate = if is_new_input { learning_rate } else { 0.25 * learning_rate };

        let nodes = once((*coord, weights, learning_rate))
            .chain(node.neighbours(self, radius).filter_map(|(coord, offset)| coord.map(|coord| (coord, offset))).map(
                |(coord, offset)| {
                    let distance = offset.0.abs() + offset.1.abs();
                    let learning_rate = learning_rate / distance as Float;
                    (coord, weights, learning_rate)
                },
            ))
            .collect::<Vec<_>>();

        nodes.into_iter().for_each(|(coord, weights, learning_rate)| {
            if let Some(node) = self.nodes.get_mut(&coord) {
                node.adjust(weights, learning_rate);
                self.min_max_weights.update(node.weights.as_slice());
            }
        })
    }

    /// Gets a mutable reference for node with given coordinate.
    pub(super) fn get_mut(&mut self, coord: &Coordinate) -> Option<&mut Node<I, S>> {
        self.nodes.get_mut(coord)
    }

    /// Inserts new neighbors if necessary.
    pub(super) fn insert(&mut self, context: &C, coord: Coordinate, weights: &[Float]) {
        self.min_max_weights.update(weights);
        self.nodes.insert(coord, self.create_node(context, coord, weights, 0.));
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
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Creates a new node for given data.
    fn create_node(&self, context: &C, coord: Coordinate, weights: &[Float], error: Float) -> Node<I, S> {
        Node::new(coord, weights, error, self.rebalance_memory, self.storage_factory.eval(context))
    }

    /// Creates nodes for initial topology.
    fn create_initial_nodes(
        context: &C,
        data: Vec<I>,
        rebalance_memory: usize,
        storage_factory: &F,
        noise: Noise,
    ) -> GenericResult<(NodeHashMap<I, S>, MinMaxWeights)> {
        // sample size is 10% of data, bounded between 4-16 nodes
        let sample_size = (data.len() as f64 * 0.1).ceil() as usize;
        let sample_size = sample_size.clamp(4, 16);

        // initialize min max weights from data
        let dimension = data[0].weights().len();
        let mut min_max = MinMaxWeights::new(dimension);
        data.iter().for_each(|i| min_max.update(i.weights()));

        let initial_node_indices = Self::select_initial_samples(&data, sample_size, &min_max, noise.random())
            .ok_or_else(|| GenericError::from("cannot select initial samples"))?;

        // create initial node coordinates and data assignments (by index)
        let grid_size = (initial_node_indices.len() as f64).sqrt().ceil() as i32;
        let mut node_assignments: HashMap<Coordinate, Vec<usize>> = initial_node_indices
            .iter()
            .enumerate()
            .map(|(grid_idx, &data_idx)| {
                (data_idx, Coordinate((grid_idx as i32) % grid_size, (grid_idx as i32) / grid_size))
            })
            .collect_group_by_key(|(_, coord)| *coord)
            .into_iter()
            .map(|(coord, items)| (coord, items.into_iter().map(|(idx, _)| idx).collect()))
            .collect();

        // assign remaining data points to initial respective coordinates based on relative distance
        for (idx, item) in data.iter().enumerate() {
            if !initial_node_indices.contains(&idx) {
                let get_distance_fn = |coord| {
                    let init_idx = node_assignments[coord][0];
                    euclidian_distance(data[init_idx].weights(), item.weights(), &min_max)
                };

                node_assignments
                    .keys()
                    .min_by(|&left, &right| get_distance_fn(left).total_cmp(&get_distance_fn(right)))
                    .cloned()
                    .and_then(|closest_coord| node_assignments.get_mut(&closest_coord))
                    .ok_or_else(|| GenericError::from("cannot find closest node"))?
                    .push(idx);
            }
        }

        // first pass: create nodes using assignments without data yet (as it is not cloneable and we need to keep indices valid)
        let mut nodes = NodeHashMap::default();
        for (&coord, indices) in node_assignments.iter() {
            let init_idx = indices[0];
            let weights: Vec<Float> = data[init_idx].weights().iter().map(|&v| noise.generate(v)).collect();
            let node = Node::new(coord, &weights, 0., rebalance_memory, storage_factory.eval(context));

            nodes.insert(coord, node);
        }

        // second pass: populate nodes with data drained
        for (idx, item) in data.into_iter().enumerate() {
            let node = node_assignments
                .iter()
                .find(|(_, indices)| indices.contains(&idx))
                .map(|(coord, _)| coord)
                .and_then(|coord| nodes.get_mut(coord))
                .ok_or_else(|| GenericError::from("cannot find node for data"))?;

            node.storage.add(item);
        }

        Ok((nodes, min_max))
    }

    /// Selects initial samples (represented as index in data).
    fn select_initial_samples(
        data: &[I],
        sample_size: usize,
        min_max: &MinMaxWeights,
        random: &dyn Random,
    ) -> Option<Vec<usize>> {
        let mut selected_indices = Vec::with_capacity(sample_size);

        // select first sample randomly
        selected_indices.push(random.uniform_int(0, data.len() as i32 - 1) as usize);

        // Select remaining samples maximizing distance
        let dist_fn = |selected_indices: &Vec<usize>, idx: usize| {
            selected_indices
                .iter()
                .map(|&sel_idx| euclidian_distance(data[sel_idx].weights(), data[idx].weights(), min_max))
                .min_by(|a, b| a.total_cmp(b))
                .unwrap_or_default()
        };
        while selected_indices.len() < sample_size {
            let next_idx = (0..data.len())
                .filter(|i| !selected_indices.contains(i))
                .max_by(|&i, &j| dist_fn(&selected_indices, i).total_cmp(&dist_fn(&selected_indices, j)))?;

            selected_indices.push(next_idx);
        }

        Some(selected_indices)
    }

    /// Returns a distance between weights.
    pub(crate) fn distance(&self, left: &[Float], right: &[Float]) -> Float {
        euclidian_distance(left, right, &self.min_max_weights)
    }

    /// Returns normalized weights.
    pub(crate) fn normalize<'a>(&'a self, values: &'a [Float]) -> impl Iterator<Item = Float> + 'a {
        normalize(values, &self.min_max_weights)
    }
}

fn compare_input<I: Input>(left: &I, right: &I) -> Ordering {
    (left.weights().iter())
        .zip(right.weights().iter())
        .map(|(lhs, rhs)| lhs.total_cmp(rhs))
        .find(|ord| *ord != Ordering::Equal)
        .unwrap_or(Ordering::Equal)
}

fn normalize<'a>(values: &'a [Float], min_max: &'a MinMaxWeights) -> impl Iterator<Item = Float> + 'a {
    values.iter().zip(min_max.iter()).map(|(&v, (min, max))| if max != min { (v - min) / (max - min) } else { 0. })
}

fn euclidian_distance(left: &[Float], right: &[Float], min_max: &MinMaxWeights) -> Float {
    let left_iter = normalize(left, min_max);
    let right_iter = normalize(right, min_max);

    // TODO allow to pass custom distance function
    left_iter.zip(right_iter).map(|(a, b)| (a - b).powi(2)).sum::<Float>().sqrt()
}
