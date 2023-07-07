#[cfg(test)]
#[path = "../../../tests/unit/algorithms/gsom/state_test.rs"]
mod state_test;

use super::*;
use crate::algorithms::gsom::Coordinate;
use std::fmt::{Display, Formatter, Result, Write};
use std::ops::Range;

/// Represents state of the network.
pub struct NetworkState {
    /// Shape of the network as (rows, cols, num of weights).
    pub shape: (Range<i32>, Range<i32>, usize),
    /// Mean of node distance.
    pub mean_distance: f64,
    /// Nodes of the network.
    pub nodes: Vec<NodeState>,
}

/// Contains information about network node state.
pub struct NodeState {
    /// Node coordinate in network.
    pub coordinate: (i32, i32),
    /// Unified distance to neighbors.
    pub unified_distance: f64,
    /// Distance between weights of individual and node weights.
    pub node_distance: Option<f64>,
    /// Node weights.
    pub weights: Vec<f64>,
    /// Total hits.
    pub total_hits: usize,
    /// Last hits.
    pub last_hits: usize,
    /// A dump of underlying node's storage.
    pub dump: String,
}

/// Gets network state.
pub fn get_network_state<I, S, F>(network: &Network<I, S, F>) -> NetworkState
where
    I: Input,
    S: Storage<Item = I>,
    F: StorageFactory<I, S>,
{
    let ((x_min, x_max), (y_min, y_max)) = get_network_shape(network);

    let mean_distance = network.mean_distance();

    let nodes = network
        .get_nodes()
        .map(|node| {
            let mut dump = String::new();
            write!(dump, "{}", node.storage).unwrap();

            NodeState {
                coordinate: (node.coordinate.0, node.coordinate.1),
                unified_distance: node.unified_distance(network, 1),
                node_distance: node.node_distance(),
                weights: node.weights.clone(),
                total_hits: node.total_hits,
                last_hits: node.get_last_hits(network.get_current_time()),
                dump,
            }
        })
        .collect::<Vec<_>>();

    let dim = nodes.first().map_or(0, |node| node.weights.len());

    NetworkState { shape: (x_min..x_max, y_min..y_max, dim), nodes, mean_distance }
}

/// Gets network's shape: min-max coordinate indices.
pub fn get_network_shape<I, S, F>(network: &Network<I, S, F>) -> ((i32, i32), (i32, i32))
where
    I: Input,
    S: Storage<Item = I>,
    F: StorageFactory<I, S>,
{
    network.get_coordinates().fold(
        ((i32::MAX, i32::MIN), (i32::MAX, i32::MIN)),
        |((x_min, x_max), (y_min, y_max)), Coordinate(x, y)| {
            ((x_min.min(x), x_max.max(x)), (y_min.min(y), y_max.max(y)))
        },
    )
}

impl Display for NetworkState {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // NOTE serialize state in simple representation which can be embedded
        // to json as string and then easily parsed.
        let nodes = self.nodes.iter().fold(String::new(), |mut res, n| {
            let (x, y) = n.coordinate;
            let weights = n.weights.iter().map(|w| format!("{w:.7}")).collect::<Vec<_>>().join(",");

            write!(
                &mut res,
                "({},{},{:.7},{},{},[{}],{}),",
                x, y, n.unified_distance, n.total_hits, n.last_hits, weights, n.dump
            )
            .unwrap();

            res
        });

        write!(
            f,
            "({},{},{},{},{},[{}])",
            self.shape.0.start, self.shape.0.end, self.shape.1.start, self.shape.1.end, self.shape.2, nodes
        )
    }
}
