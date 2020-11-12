use super::{Input, Network, Storage};
use crate::algorithms::gsom::Coordinate;
use std::i32::{MAX, MIN};
use std::ops::Range;

/// Represents state of the network.
pub struct NetworkState {
    /// Shape of the network as (rows, cols, num of weights).
    pub shape: (Range<i32>, Range<i32>, usize),
    /// Nodes of the network.
    pub nodes: Vec<NodeState>,
}

/// Contains information about network node state.
pub struct NodeState {
    /// Node coordinate in network.
    pub coordinate: (i32, i32),
    /// Unified distance to neighbors.
    pub unified_distance: f64,
    /// Node weights.
    pub weights: Vec<f64>,
}

/// Gets network state.
pub fn get_network_state<I: Input, S: Storage<Item = I>>(network: &Network<I, S>) -> NetworkState {
    let ((x_min, x_max), (y_min, y_max)) = network.get_coordinates().fold(
        ((MAX, MIN), (MAX, MIN)),
        |((x_min, x_max), (y_min, y_max)), Coordinate(x, y)| {
            ((x_min.min(x), x_max.max(x)), (y_min.min(y), y_max.max(y)))
        },
    );

    let nodes = network
        .get_nodes()
        .map(|node| {
            let node = node.read().unwrap();

            let (sum, count) = node.topology.neighbours().fold((0., 0), |(sum, count), nn| {
                let distance = node.storage.distance(node.weights.as_slice(), nn.read().unwrap().weights.as_slice());
                (sum + distance, count + 1)
            });

            NodeState {
                coordinate: (node.coordinate.0, node.coordinate.1),
                unified_distance: if count > 0 { sum / count as f64 } else { 0. },
                weights: node.weights.clone(),
            }
        })
        .collect::<Vec<_>>();

    let dim = nodes.first().map_or(0, |node| node.weights.len());

    NetworkState { shape: (x_min..x_max, y_min..y_max, dim), nodes }
}
