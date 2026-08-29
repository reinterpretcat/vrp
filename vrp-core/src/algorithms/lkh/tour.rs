#[cfg(test)]
#[path = "../../../tests/unit/algorithms/lkh/tour_test.rs"]
mod tour_test;

use super::*;
use crate::utils::Either;
use std::iter::{empty, once};

/// A tour is a sequence of nodes that visits each node exactly once.
pub struct Tour {
    path: Path,
    edges: EdgeSet,
    indices: Option<TinyVec<[usize; 32]>>,
}

/// Reusable buffers for path relinking.
#[derive(Default)]
pub(crate) struct TourScratch {
    edges: Vec<Edge>,
    successors: Vec<(Node, Node)>,
    visited: Vec<Node>,
    path: Path,
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
        let indices = path.iter().max().filter(|&&max| max < path.len().saturating_mul(2)).map(|&max| {
            let mut indices = TinyVec::<[usize; 32]>::new();
            indices.resize(max + 1, usize::MAX);
            path.iter().enumerate().for_each(|(index, &node)| {
                if indices[node] == usize::MAX {
                    indices[node] = index;
                }
            });
            indices
        });

        Tour { path, edges, indices }
    }

    /// Returns true if the given edge is in the tour.
    pub fn contains(&self, edge: &Edge) -> bool {
        self.edges.contains(edge)
    }

    /// Returns the index of the given node in the tour.
    pub fn index_of(&self, node: Node) -> Option<usize> {
        match &self.indices {
            Some(indices) => indices.get(node).copied().filter(|&index| index != usize::MAX),
            None => self.path.iter().position(|&candidate| candidate == node),
        }
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
    pub(crate) fn try_path(&self, broken: &EdgeSet, joined: &EdgeSet, scratch: &mut TourScratch) -> Option<Path> {
        let TourScratch { edges, successors, visited, path } = scratch;

        edges.clear();
        edges.extend(self.edges.iter().copied());
        broken.iter().for_each(|edge| {
            if let Ok(index) = edges.binary_search(edge) {
                edges.remove(index);
            }
        });
        joined.iter().copied().for_each(|edge| {
            if let Err(index) = edges.binary_search(&edge) {
                edges.insert(index, edge);
            }
        });

        // if we do not have enough edges, we cannot form a tour, but this should not happen in LKH.
        if edges.len() < self.len() {
            return None;
        }

        // NOTE: get start location, assume that the tour starts always from it (e.g. from depot).
        let start_node = self.index_of(self.path[0])?;

        successors.clear();
        let mut node = start_node;
        while !edges.is_empty() {
            if let Some(index) = edges.iter().position(|&(i, j)| i == node || j == node) {
                let edge = edges.remove(index);
                let next_node = if edge.0 == node { edge.1 } else { edge.0 };
                if let Some((_, successor)) = successors.iter_mut().find(|(current, _)| *current == node) {
                    *successor = next_node;
                } else {
                    successors.push((node, next_node));
                }
                node = next_node;
            } else {
                break;
            }
        }

        // similarly, if not every node has a successor, tour is invalid
        if successors.len() != self.len() {
            return None;
        }

        visited.clear();
        visited.push(start_node);

        path.clear();
        path.push(start_node);
        let mut node = start_node;
        while let Some((_, next)) = successors.iter().find(|(current, _)| *current == node) {
            if visited.contains(next) {
                break;
            }

            node = *next;
            visited.push(node);
            path.push(node);
        }

        if path.len() == self.len() { Some(path.clone()) } else { None }
    }
}
