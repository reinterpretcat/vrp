#[cfg(test)]
#[path = "../../../tests/unit/algorithms/lkh/tour_test.rs"]
mod tour_test;

use super::*;
use crate::utils::Either;
use std::{
    collections::{HashMap, HashSet},
    iter::{empty, once},
};

/// A tour is a sequence of nodes that visits each node exactly once.
pub struct Tour {
    path: Path,
    edges: EdgeSet,
}

impl Tour {
    /// Creates a new tour from a sequence of nodes.
    pub fn new<I>(path: I) -> Self
    where
        I: IntoIterator<Item = Node>,
    {
        let path: Path = path.into_iter().collect();
        let edges = path
            .windows(2)
            .map(|w| (w[0], w[1]))
            .chain(path.last().copied().zip(path.first().copied()))
            .map(|(from, to)| make_edge(from, to))
            .collect();

        Tour { path, edges }
    }

    /// Returns true if the given edge is in the tour.
    pub fn contains(&self, edge: &Edge) -> bool {
        self.edges.contains(edge)
    }

    /// Returns the index of the given node in the tour.
    pub fn index_of(&self, node: Node) -> Option<usize> {
        self.path.iter().position(|&n| n == node)
    }

    /// Returns neighbours around of a given node.
    pub fn around(&self, node: Node) -> impl Iterator<Item = Node> {
        self.index_of(node)
            .map(|index| {
                let pred = if index == 0 { self.path.len() - 1 } else { index - 1 };
                let succ = (index + 1) % self.path.len();
                (self.path[pred], self.path[succ])
            })
            .map(|(pred, succ)| Either::Left(once(pred).chain(once(succ))))
            .unwrap_or_else(|| Either::Right(empty()))
    }

    /// Returns an iterator over the nodes in the tour.
    pub fn path(&self) -> impl Iterator<Item = Node> + '_ {
        self.path.iter().copied()
    }

    /// Returns the length of the tour.
    pub fn len(&self) -> usize {
        self.path.len()
    }

    /// Applies modifications on the copy of existing tour's path and returns a new path if it is valid.
    /// Please note that validity of the path is checked only from TSP prospective.
    pub(crate) fn try_path(&self, broken: &EdgeSet, joined: &EdgeSet) -> Option<Path> {
        let mut edges: EdgeSet = self.edges.difference(broken).cloned().chain(joined.iter().cloned()).collect();

        // if we do not have enough edges, we cannot form a tour, but this should not happen in LKH.
        if edges.len() < self.len() {
            return None;
        }

        // NOTE: get start location, assume that the tour starts always from it (e.g. from depot).
        let start_node = self.index_of(self.path[0])?;

        let mut successors = HashMap::new();
        let mut node = start_node;
        while !edges.is_empty() {
            if let Some(&edge) = edges.iter().find(|&&(i, j)| i == node || j == node) {
                let next_node = if edge.0 == node { edge.1 } else { edge.0 };
                successors.insert(node, next_node);
                edges.remove(&edge);
                node = next_node;
            } else {
                break;
            }
        }

        // similarly, if not every node has a successor, tour is invalid
        if successors.len() != self.len() {
            return None;
        }

        let mut visited = HashSet::with_capacity(self.len());
        visited.insert(start_node);

        let new_tour: Path = std::iter::successors(Some(start_node), |&node| {
            successors.get(&node).copied().and_then(|next| {
                if visited.contains(&next) {
                    None
                } else {
                    visited.insert(next);
                    Some(next)
                }
            })
        })
        .collect();

        if new_tour.len() == self.len() { Some(new_tour) } else { None }
    }
}
