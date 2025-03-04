/// The modified Lin-Kernighan-Helsgaun algorithm for the Traveling Salesman Problem.
///
/// This implementation is based on the Lin-Kernighan-Helsgaun algorithm
/// implementation from https://gitlab.com/Soha/local-tsp
///
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

/// A set of edges.
pub(crate) type EdgeSet = BTreeSet<Edge>;

/// Represents graph structure with weighted edges and neighborhood relationships.
pub trait AdjacencySpec {
    /// Returns transition cost for the given edge.
    fn cost(&self, edge: &Edge) -> Cost;

    /// Returns the neighbours of a node.
    fn neighbours(&self, node: Node) -> &[Node];
}

/// Creates an edge from a pair of nodes.
pub(crate) fn make_edge(i: Node, j: Node) -> Edge {
    if i < j {
        Edge::from((i, j))
    } else {
        Edge::from((j, i))
    }
}

/// Creates a set of edges from an iterator of edges.
pub(crate) fn make_edge_set<I>(edges: I) -> EdgeSet
where
    I: IntoIterator<Item = (Node, Node)>,
{
    edges.into_iter().map(|(i, j)| make_edge(i, j)).collect()
}
