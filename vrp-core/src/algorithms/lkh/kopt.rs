#[cfg(test)]
#[path = "../../../tests/unit/algorithms/lkh/kopt_test.rs"]
mod kopt_test;

use super::*;
use crate::utils::Either;
use std::iter::once;

/// Implements the k-opt move operation for tour improvement.
///
/// A k-opt move removes k edges from a tour and reconnects the resulting paths
/// in a different way to potentially create a shorter tour.
///
/// # Type Parameters
///
/// * `T` - The adjacency specification type that provides distance information between nodes
pub(crate) struct KOpt<T> {
    adjacency: T,
    solutions: Vec<Path>,
}

impl<T> KOpt<T>
where
    T: AdjacencySpec,
{
    /// Creates a new instance of [KOpt].
    pub fn new(adjacency: T) -> Self {
        KOpt { adjacency, solutions: Vec::default() }
    }

    /// Tries to optimize a given path using modified Lin-Kernighan-Helsgaun algorithm.
    /// Returns discovered solutions in the order of their improvement.
    pub fn optimize(mut self, path: Path) -> Vec<Path> {
        self.solutions.push(path);

        loop {
            if let Some(path) = self.solutions.last().and_then(|p| self.improve(p.iter().copied())) {
                self.solutions.push(path);
            } else {
                break;
            }
        }

        self.solutions
    }

    /// Attempts to improve a given tour using the Lin-Kernighan-Helsgaun algorithm.
    ///
    /// # Arguments
    ///
    /// * `path` - An iterator over nodes representing the initial tour
    ///
    /// # Returns
    ///
    /// Some([`Path`]) if an improved tour is found, None otherwise.
    fn improve<I>(&self, path: I) -> Option<Path>
    where
        I: IntoIterator<Item = Node>,
    {
        let tour = Tour::new(path);

        for t1 in tour.path() {
            let around: BTreeSet<_> = tour.around(t1).collect();
            for &t2 in around.iter() {
                let broken = make_edge_set(once((t1, t2)));

                // initial savings
                let gain = self.adjacency.cost(&(t1, t2));

                let closest = self.find_closest(&tour, t2, gain, &broken, &EdgeSet::new());

                // number of neighbours to try
                let mut tries = 5;

                for (t3, (_, gi)) in closest {
                    // make sure that the new node is none of t_1's neighbours so it does not belong to the tour.
                    if around.contains(&t3) {
                        continue;
                    }

                    let joined = make_edge_set(once((t2, t3)));

                    // the positive Gi is taken care of by `find_closest()`
                    match self.choose_x(&tour, t1, t3, gi, &broken, &joined) {
                        // return to Step 2 (initial loop)
                        Some(path) => return Some(path),
                        // else try the other options
                        None => {
                            tries -= 1;
                            // explored enough nodes, change t_2
                            if tries == 0 {
                                break;
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Finds and sorts neighboring nodes by their potential gain, computing partial improvements.
    ///
    /// # Arguments
    ///
    /// * `tour` - The current tour to optimize
    /// * `t2i` - The node to relink from
    /// * `gain` - The current cumulative gain (Gi)
    /// * `broken` - Set of edges to be removed from tour (X)
    /// * `joined` - Set of edges to be added to tour (Y)
    ///
    /// # Returns
    ///
    /// A vector of tuples (`Node`, (`Cost`, `Cost`)) sorted by potential improvement,
    /// where each tuple contains a neighboring node and its associated costs.
    fn find_closest(
        &self,
        tour: &Tour,
        t2i: Node,
        gain: Cost,
        broken: &EdgeSet,
        joined: &EdgeSet,
    ) -> Vec<(Node, (Cost, Cost))> {
        let mut neighbours = HashMap::new();

        // create the neighbours of t_2i
        for &node in self.adjacency.neighbours(t2i) {
            let yi = make_edge(t2i, node);
            let gi = gain - self.adjacency.cost(&(t2i, node));

            // any new edge has to have a positive running sum, not be a broken
            // edge and not belong to the tour.
            if gi <= 0. || broken.contains(&yi) || tour.contains(&yi) {
                continue;
            }

            for succ in tour.around(node) {
                let xi = make_edge(node, succ);

                // check that "x_i+1 exists"
                if !broken.contains(&xi) && !joined.contains(&xi) {
                    let diff = self.adjacency.cost(&(node, succ)) - self.adjacency.cost(&(t2i, node));

                    neighbours
                        .entry(node)
                        .and_modify(|(d, g)| {
                            *d = diff;
                            if diff < *d {
                                *g = gi;
                            }
                        })
                        .or_insert((diff, gi));
                }
            }
        }

        // sort by diff
        let mut neighbours = neighbours.into_iter().collect::<Vec<_>>();
        neighbours.sort_by(|(_, (a, _)), (_, (b, _))| b.total_cmp(a));

        neighbours
    }

    /// Attempts to find and omit an edge from the tour that leads to an improvement.
    ///
    /// # Arguments
    ///
    /// * `tour` - The current tour to optimize
    /// * `t1` - The starting node for k-opt move
    /// * `last` - The tail node of the last added edge (t_2i-1)
    /// * `gain` - The current cumulative gain (Gi)
    /// * `broken` - Set of edges to be removed from tour (X)
    /// * `joined` - Set of edges to be added to tour (Y)
    ///
    /// # Returns
    ///
    /// Some([`Path`]) if an improved tour is found, None otherwise.
    fn choose_x(
        &self,
        tour: &Tour,
        t1: Node,
        last: Node,
        gain: Cost,
        broken: &EdgeSet,
        joined: &EdgeSet,
    ) -> Option<Path> {
        let nodes_around = if broken.len() == 4 {
            // NOTE: assume that there are two neighbours around
            let mut around = tour.around(last);
            let Some((pred, succ)) = around.next().zip(around.next()) else {
                return None;
            };

            // give priority to the longest edge for x_4
            if self.adjacency.cost(&(pred, last)) > self.adjacency.cost(&(succ, last)) {
                Either::Left(once(pred))
            } else {
                Either::Left(once(succ))
            }
        } else {
            Either::Right(tour.around(last))
        };

        for t2i in nodes_around {
            let xi = make_edge(last, t2i);

            // verify that X and Y are disjoint, though also need to check
            // that we are not including an x_i again for some reason.
            if joined.contains(&xi) || broken.contains(&xi) {
                return None;
            }

            let yi = make_edge(t2i, t1);
            let added = joined.iter().cloned().chain(once(yi)).collect();
            let removed = broken.iter().cloned().chain(once(xi)).collect();

            // get gain at current iteration and try to relink the tour
            let gi = gain + self.adjacency.cost(&(last, t2i));
            let relink = gi - self.adjacency.cost(&(t2i, t1));

            if relink > 0. {
                // Try to find valid path
                match tour.try_path(&removed, &added) {
                    // save the current solution on caller site if the tour is better
                    Some(new_path) if !self.is_known_path(&new_path) => return Some(new_path),
                    // skip already found tour
                    Some(_) => return None,
                    // the current solution does not form a valid tour
                    None if added.len() > 2 => continue,
                    _ => {}
                }
            }

            // pass on the newly "removed" edge but not the relink
            return self.choose_y(tour, t1, t2i, gi, &removed, joined);
        }

        None
    }

    /// Attempts to add an edge to the tour that leads to an improvement.
    ///
    /// # Arguments
    ///
    /// * `tour` - The current tour to optimize
    /// * `t1` - The starting node for k-opt move
    /// * `t2i` - The tail node of the last removed edge
    /// * `gain` - The current cumulative gain (Gi)
    /// * `broken` - Set of edges to be removed from tour (X)
    /// * `joined` - Set of edges to be added to tour (Y)
    ///
    /// # Returns
    ///
    /// Some([`Path`]) if an improved tour is found, None otherwise.
    fn choose_y(
        &self,
        tour: &Tour,
        t1: Node,
        t2i: Node,
        gain: Cost,
        broken: &EdgeSet,
        joined: &EdgeSet,
    ) -> Option<Path> {
        let closest = self.find_closest(tour, t2i, gain, broken, joined);
        let max_tries = if broken.len() == 2 { 5 } else { 1 };

        closest.into_iter().take(max_tries).find_map(|(node, (_, gi))| {
            let yi = make_edge(t2i, node);
            let added = joined.iter().cloned().chain(once(yi)).collect();

            self.choose_x(tour, t1, node, gi, broken, &added)
        })
    }

    /// Checks if the given path is already known.
    fn is_known_path(&self, path: &[Node]) -> bool {
        self.solutions.iter().find(|&p| p.iter().zip(path.iter()).all(|(&a, &b)| a == b)).is_some()
    }
}
