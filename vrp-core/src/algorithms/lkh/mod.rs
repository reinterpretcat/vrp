//! The modified Lin-Kernighan-Helsgaun algorithm for the Traveling Salesman Problem.
//!
//! This implementation is based on the Lin-Kernighan-Helsgaun algorithm
//! implementation from https://gitlab.com/Soha/local-tsp

use std::iter::FromIterator;
use tinyvec::TinyVec;

mod tour;
use self::tour::{Tour, TourScratch};

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

    /// Returns the unique neighbours of a node.
    fn neighbours(&self, node: Node) -> &[Node];
}

/// Optimizes a path using modified Lin-Kernighan-Helsgaun algorithm.
pub fn lkh_optimize<T>(adjacency: T, path: Path) -> Vec<Path>
where
    T: AdjacencySpec,
{
    KOpt::new(adjacency).optimize(path)
}

/// A sorted set of edges.
///
/// LKH usually keeps only a few broken and joined edges. Keeping these sets inline avoids
/// allocating a tree node for every edge while retaining deterministic set iteration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct EdgeSet {
    edges: TinyVec<[Edge; 8]>,
}

impl EdgeSet {
    fn new() -> Self {
        Self::default()
    }

    fn len(&self) -> usize {
        self.edges.len()
    }

    fn contains(&self, edge: &Edge) -> bool {
        self.edges.binary_search(edge).is_ok()
    }

    fn insert(&mut self, edge: Edge) -> bool {
        if let Err(index) = self.edges.binary_search(&edge) {
            self.edges.insert(index, edge);
            true
        } else {
            false
        }
    }

    fn iter(&self) -> std::slice::Iter<'_, Edge> {
        self.edges.iter()
    }

    fn with_edge(&self, edge: Edge) -> Self {
        let mut result = self.clone();
        result.insert(edge);

        result
    }
}

impl FromIterator<Edge> for EdgeSet {
    fn from_iter<I: IntoIterator<Item = Edge>>(iter: I) -> Self {
        let mut edges = iter.into_iter().collect::<TinyVec<[Edge; 8]>>();
        edges.sort_unstable();
        let mut index = 1;
        while index < edges.len() {
            if edges[index - 1] == edges[index] {
                edges.remove(index);
            } else {
                index += 1;
            }
        }

        Self { edges }
    }
}

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
