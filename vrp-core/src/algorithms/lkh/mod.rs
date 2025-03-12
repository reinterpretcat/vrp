//! The modified Lin-Kernighan-Helsgaun algorithm for the Traveling Salesman Problem.
//!
//! This implementation is based on the Lin-Kernighan-Helsgaun algorithm
//! implementation from https://gitlab.com/Soha/local-tsp

use std::collections::{BTreeSet, HashMap};

mod tour;
use self::tour::Tour;

mod kopt;
use self::kopt::KOpt;

/// A node is a unique identifier for a location in a tour.
pub type Node = usize;

/// An edge is a pair of nodes that are connected in a tour.
pub type Edge = (usize, usize);

/// A path is a sequence of nodes that are connected in a tour.
pub type Path = Vec<Node>;

/// Represents the cost of a transition.
pub type Cost = f64;

/// Represents graph structure with weighted edges and neighborhood relationships.
pub trait AdjacencySpec {
    /// Returns transition cost for the given edge.
    fn cost(&self, edge: &Edge) -> Cost;

    /// Returns the neighbours of a node.
    fn neighbours(&self, node: Node) -> &[Node];
}

/// Optimizes a path using modified Lin-Kernighan-Helsgaun algorithm.
pub fn lkh_optimize<T>(adjacency: T, path: Path) -> Vec<Path>
where
    T: AdjacencySpec,
{
    KOpt::new(adjacency).optimize(path)
}

/// A set of edges.
type EdgeSet = BTreeSet<Edge>;

/// Creates an edge from a pair of nodes.
fn make_edge(i: Node, j: Node) -> Edge {
    if i < j { Edge::from((i, j)) } else { Edge::from((j, i)) }
}

/// Creates a set of edges from an iterator of edges.
fn make_edge_set<I>(edges: I) -> EdgeSet
where
    I: IntoIterator<Item = (Node, Node)>,
{
    edges.into_iter().map(|(i, j)| make_edge(i, j)).collect()
}
