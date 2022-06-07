//! Borrowed from https://github.com/ricardopieper/fdeb-rs/blob/master/src/fdeb.rs

#[cfg(test)]
#[path = "../../../tests/unit/solver/vrp/fdeb_test.rs"]
mod fdeb_test;

use super::{GraphEdge, GraphNode};
use crate::DataGraph;
use std::collections::HashMap;

const EPS: f64 = 1e-6;
const P_INITIAL: usize = 1;
const K: f64 = 0.1; // global bundling constant controlling edge stiffness
const COMPATIBILITY_THRESHOLD: f64 = 0.6;
const S_INITIAL: f64 = 0.1; // init. distance to move points
const P_RATE: usize = 2; // subdivision rate increase
const C: i32 = 8; // number of cycles to perform
const I_INITIAL: usize = 90; // init. number of iterations for cycle
const I_RATE: f64 = 2.0 / 3.0; // rate at which iteration number decreases i.e. 2/3

pub struct Fdeb {
    pub graph: DataGraph,
}

impl Fdeb {
    pub fn new(graph: DataGraph) -> Fdeb {
        let mut graph = graph;
        let edges = graph.edges;
        graph.edges = Fdeb::filter_self_loops(&graph.nodes, edges);

        Fdeb { graph }
    }

    fn vector_dot_product(&self, p: &GraphNode, q: &GraphNode) -> f64 {
        p.x * q.x + p.y * q.y
    }

    //
    fn edge_as_vector(&self, p: &GraphEdge) -> GraphNode {
        GraphNode {
            x: self.graph.nodes[p.target].x - self.graph.nodes[p.source].x,
            y: self.graph.nodes[p.target].y - self.graph.nodes[p.source].y,
        }
    }

    //
    fn edge_length(&self, node: &GraphEdge) -> f64 {
        // handling nodes that are on the same location, so that K/edge_length != Inf
        if (self.graph.nodes[node.source].x - self.graph.nodes[node.target].x).abs() < EPS
            && (self.graph.nodes[node.source].y - self.graph.nodes[node.target].y).abs() < EPS
        {
            EPS
        } else {
            self.euclidean_distance(&self.graph.nodes[node.source], &self.graph.nodes[node.target])
        }
    }

    //
    fn edge_midpoint(&self, e: &GraphEdge) -> GraphNode {
        let middle_x = (self.graph.nodes[e.source].x + self.graph.nodes[e.target].x) / 2.0;
        let middle_y = (self.graph.nodes[e.source].y + self.graph.nodes[e.target].y) / 2.0;

        GraphNode { x: middle_x, y: middle_y }
    }

    fn euclidean_distance(&self, p: &GraphNode, q: &GraphNode) -> f64 {
        ((p.x - q.x).powi(2) + (p.y - q.y).powi(2)).sqrt()
    }

    //
    fn _compute_divided_edge_length_approx(&self, subdivision_points_for_edge: &[GraphNode]) -> f64 {
        self.euclidean_distance(
            subdivision_points_for_edge.first().unwrap(),
            subdivision_points_for_edge.last().unwrap(),
        )
    }

    //
    fn compute_divided_edge_length(&self, subdivision_points_for_edge: &[GraphNode]) -> f64 {
        let from_first = subdivision_points_for_edge.iter().skip(1);
        let zipped = from_first.zip(subdivision_points_for_edge.iter());
        zipped.map(|(edge1, edge2)| self.euclidean_distance(edge1, edge2)).sum()
    }

    //
    fn project_point_on_line(&self, p: &GraphNode, q_source: &GraphNode, q_target: &GraphNode) -> GraphNode {
        let l = (q_target.x - q_source.x).powi(2) + (q_target.y - q_source.y).powi(2);

        let r = ((q_source.y - p.y) * (q_source.y - q_target.y) - (q_source.x - p.x) * (q_target.x - q_source.x)) / l;

        GraphNode { x: (q_source.x + r * (q_target.x - q_source.x)), y: (q_source.y + r * (q_target.y - q_source.y)) }
    }

    //
    fn initialize_edge_subdivisions(&self) -> Vec<Vec<GraphNode>> {
        let mut subdivision_points_for_edges = Vec::<Vec<GraphNode>>::with_capacity(self.graph.edges.len());

        for _ in 0..self.graph.edges.len() {
            subdivision_points_for_edges.push(Vec::<GraphNode>::new());
        }

        subdivision_points_for_edges
    }

    fn initialize_compatibility_lists(&self) -> HashMap<usize, Vec<usize>> {
        let mut compatibility_list_for_edge = HashMap::with_capacity(self.graph.edges.len());
        for i in 0..self.graph.edges.len() {
            compatibility_list_for_edge.insert(i, Vec::<usize>::new());
        }
        compatibility_list_for_edge
    }

    //
    fn apply_spring_force(&self, subdivision_points_for_edge: &[GraphNode], i: usize, k_p: f64) -> GraphNode {
        if subdivision_points_for_edge.len() < 3 {
            GraphNode { x: 0.0, y: 0.0 }
        } else {
            let prev = &subdivision_points_for_edge[i - 1];
            let next = &subdivision_points_for_edge[i + 1];
            let crnt = &subdivision_points_for_edge[i];
            let x = prev.x - crnt.x + next.x - crnt.x;
            let y = prev.y - crnt.y + next.y - crnt.y;

            GraphNode { x: x * k_p, y: y * k_p }
        }
    }

    //
    fn apply_electrostatic_force(
        &self,
        subdivision_points_for_edge: &[Vec<GraphNode>],
        compatible_edges_list: &[usize],
        i: usize,
        e_idx: usize,
    ) -> GraphNode {
        if e_idx > subdivision_points_for_edge.len() - 1 || i > subdivision_points_for_edge[e_idx].len() - 1 {
            GraphNode { x: 0.0, y: 0.0 }
        } else {
            let edge = &subdivision_points_for_edge[e_idx][i];

            let (x, y) = compatible_edges_list
                .iter()
                .map(|oe| {
                    if *oe > subdivision_points_for_edge.len() - 1 || i > subdivision_points_for_edge[*oe].len() - 1 {
                        (0.0, 0.0)
                    } else {
                        let edge_oe = &subdivision_points_for_edge[*oe][i];
                        let force_x = edge_oe.x - edge.x;
                        let force_y = edge_oe.y - edge.y;

                        if (force_x.abs() > EPS) || (force_y.abs() > EPS) {
                            let len = self.euclidean_distance(edge_oe, edge);
                            let diff = 1.0 / len;
                            (force_x * diff, force_y * diff)
                        } else {
                            (0.0, 0.0)
                        }
                    }
                })
                .fold((0.0, 0.0), |(acc_x, acc_y), (x, y)| (acc_x + x, acc_y + y));

            GraphNode { x, y }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn compute_forces_on_point(
        &self,
        e_idx: usize,
        s: f64,
        i: usize,
        k_p: f64,
        subdivision_points_for_edges: &[Vec<GraphNode>],
        edge_subdivisions: &[GraphNode],
        compatible_edges_list: &[usize],
    ) -> GraphNode {
        let spring_force = self.apply_spring_force(edge_subdivisions, i, k_p);
        let electrostatic_force =
            self.apply_electrostatic_force(subdivision_points_for_edges, compatible_edges_list, i, e_idx);

        GraphNode { x: s * (spring_force.x + electrostatic_force.x), y: s * (spring_force.y + electrostatic_force.y) }
    }

    //
    fn compute_forces_on_points_iterator(
        &self,
        e_idx: usize,
        p: usize,
        s: f64,
        subdivision_points_for_edges: &[Vec<GraphNode>],
        compatible_edges_list: &[usize],
    ) -> Vec<GraphNode> {
        let edge_subdivisions = &subdivision_points_for_edges[e_idx];
        let k_p = K / (self.edge_length(&self.graph.edges[e_idx]) * (p as f64 + 1.0));
        (1..=p)
            .map(move |i| {
                self.compute_forces_on_point(
                    e_idx,
                    s,
                    i,
                    k_p,
                    subdivision_points_for_edges,
                    edge_subdivisions,
                    compatible_edges_list,
                )
            })
            .collect()
    }

    //
    fn apply_resulting_forces_on_subdivision_points(
        &self,
        e_idx: usize,
        p: usize,
        s: f64,
        subdivision_points_for_edges: &[Vec<GraphNode>],
        compatible_edges_list: &[usize],
    ) -> Vec<GraphNode> {
        self.compute_forces_on_points_iterator(e_idx, p, s, subdivision_points_for_edges, compatible_edges_list)
    }

    //
    fn update_edge_divisions(&self, p: usize, subdivision_points_for_edge: &mut [Vec<GraphNode>]) {
        if p == 1 {
            let edge_subdivs = subdivision_points_for_edge.iter_mut().zip(self.graph.edges.iter());

            edge_subdivs.for_each(|(subdivisions, edge)| {
                *subdivisions = vec![
                    self.graph.nodes[edge.source].clone(),
                    self.edge_midpoint(edge),
                    self.graph.nodes[edge.target].clone(),
                ]
            });
        } else {
            let edge_subdivs = subdivision_points_for_edge.iter_mut().zip(self.graph.edges.iter());

            edge_subdivs.for_each(|(subdivisions, edge)| {
                let divided_edge_length = self.compute_divided_edge_length(subdivisions);
                let segment_length = divided_edge_length / (p + 1) as f64;
                let mut current_segment_length = segment_length;

                let mut new_subdivision_points = Vec::<GraphNode>::with_capacity(subdivisions.len() * 2);

                new_subdivision_points.push(self.graph.nodes[edge.source].clone());

                for i in 1..subdivisions.len() {
                    let subdivision = &subdivisions[i];
                    let prev_subdivision = &subdivisions[i - 1];

                    let mut old_segment_length = self.euclidean_distance(subdivision, prev_subdivision);

                    while old_segment_length > current_segment_length {
                        let percent_position = current_segment_length / old_segment_length;
                        let mut new_subdivision_point_x = prev_subdivision.x;
                        let mut new_subdivision_point_y = prev_subdivision.y;

                        new_subdivision_point_x += percent_position * (subdivision.x - prev_subdivision.x);
                        new_subdivision_point_y += percent_position * (subdivision.y - prev_subdivision.y);

                        old_segment_length -= current_segment_length;
                        current_segment_length = segment_length;

                        new_subdivision_points
                            .push(GraphNode { x: new_subdivision_point_x, y: new_subdivision_point_y });
                    }

                    current_segment_length -= old_segment_length;
                }
                new_subdivision_points.push(self.graph.nodes[edge.target].clone());

                *subdivisions = new_subdivision_points;
            })
        }
    }

    fn angle_compatibility(&self, p: &GraphEdge, q: &GraphEdge) -> f64 {
        (self.vector_dot_product(&self.edge_as_vector(p), &self.edge_as_vector(q))
            / (self.edge_length(p) * self.edge_length(q)))
        .abs()
    }

    fn scale_compatibility(&self, p: &GraphEdge, q: &GraphEdge) -> f64 {
        let lavg = (self.edge_length(p) + self.edge_length(q)) / 2.0;
        2.0 / (lavg / self.edge_length(p).min(self.edge_length(q))
            + self.edge_length(p).max(self.edge_length(q)) / lavg)
    }

    fn position_compatibility(&self, p: &GraphEdge, q: &GraphEdge) -> f64 {
        let lavg = (self.edge_length(p) + self.edge_length(q)) / 2.0;
        let mid_p = GraphNode {
            x: (self.graph.nodes[p.source].x + self.graph.nodes[p.target].x) / 2.0,
            y: (self.graph.nodes[p.source].y + self.graph.nodes[p.target].y) / 2.0,
        };
        let mid_q = GraphNode {
            x: (self.graph.nodes[q.source].x + self.graph.nodes[q.target].x) / 2.0,
            y: (self.graph.nodes[q.source].y + self.graph.nodes[q.target].y) / 2.0,
        };

        lavg / (lavg + self.euclidean_distance(&mid_p, &mid_q))
    }

    fn edge_visibility(&self, p: &GraphEdge, q: &GraphEdge) -> f64 {
        let i_0 = self.project_point_on_line(
            &self.graph.nodes[q.source],
            &self.graph.nodes[p.source],
            &self.graph.nodes[p.target],
        );
        let i_1 = self.project_point_on_line(
            &self.graph.nodes[q.target],
            &self.graph.nodes[p.source],
            &self.graph.nodes[p.target],
        ); //send actual edge points positions
        let mid_i = GraphNode { x: (i_0.x + i_1.x) / 2.0, y: (i_0.y + i_1.y) / 2.0 };

        let mid_p = GraphNode {
            x: (self.graph.nodes[p.source].x + self.graph.nodes[p.target].x) / 2.0,
            y: (self.graph.nodes[p.source].y + self.graph.nodes[p.target].y) / 2.0,
        };

        (0.0_f64).max(1.0 - 2.0 * self.euclidean_distance(&mid_p, &mid_i) / self.euclidean_distance(&i_0, &i_1))
    }

    fn visibility_compatibility(&self, p: &GraphEdge, q: &GraphEdge) -> f64 {
        self.edge_visibility(p, q).min(self.edge_visibility(q, p))
    }

    fn compatibility_score(&self, p: &GraphEdge, q: &GraphEdge) -> f64 {
        self.angle_compatibility(p, q)
            * self.scale_compatibility(p, q)
            * self.position_compatibility(p, q)
            * self.visibility_compatibility(p, q)
    }

    fn are_compatible(&self, p: &GraphEdge, q: &GraphEdge) -> bool {
        self.compatibility_score(p, q) >= COMPATIBILITY_THRESHOLD
    }

    fn compute_compatibility_lists(&self, compatibility_list_for_edge: &mut HashMap<usize, Vec<usize>>) {
        (0..self.graph.edges.len() - 1)
            .flat_map(|e| {
                (e + 1..self.graph.edges.len())
                    .filter(move |oe| self.are_compatible(&self.graph.edges[e], &self.graph.edges[*oe]))
                    .map(move |oe| (e, oe))
            })
            .collect::<Vec<(usize, usize)>>()
            .iter()
            .for_each(|(e, oe)| {
                {
                    let vec_e = compatibility_list_for_edge.get_mut(e).unwrap();
                    vec_e.push(*oe);
                }
                {
                    let vec_oe = compatibility_list_for_edge.get_mut(oe).unwrap();
                    vec_oe.push(*e);
                }
            })
    }

    //
    fn filter_self_loops(vertices: &[GraphNode], edges: Vec<GraphEdge>) -> Vec<GraphEdge> {
        fn f64_equals(x: f64, y: f64) -> bool {
            (x - y).abs() > f64::EPSILON
        }

        edges
            .into_iter()
            .filter(|e| {
                let target_x = vertices[e.target].x;
                let target_y = vertices[e.target].y;
                let source_x = vertices[e.target].x;
                let source_y = vertices[e.target].y;
                !f64_equals(source_x, target_x) || !f64_equals(source_y, target_y)
            })
            .collect()
    }
    #[inline(never)]
    fn do_cycles(
        &self,
        mut edge_subdivisions: Vec<Vec<GraphNode>>,
        compatibility_lists: &HashMap<usize, Vec<usize>>,
    ) -> Vec<Vec<GraphNode>> {
        let mut s = S_INITIAL;
        let mut i = I_INITIAL;
        let mut p = P_INITIAL;

        for _ in 0..C {
            for _ in 0..i {
                let forces: Vec<Vec<GraphNode>> = (0..self.graph.edges.len())
                    .map(|edge| {
                        self.apply_resulting_forces_on_subdivision_points(
                            edge,
                            p,
                            s,
                            &edge_subdivisions,
                            &compatibility_lists[&edge],
                        )
                    })
                    .collect();

                for edge in 0..self.graph.edges.len() {
                    let subdiv = &mut edge_subdivisions[edge];
                    let edge_forces = &forces[edge];
                    for ii in 0..p {
                        if subdiv.len() - 1 > ii + 1 && edge_forces.len() > ii {
                            subdiv[ii + 1].x += edge_forces[ii].x;
                            subdiv[ii + 1].y += edge_forces[ii].y;
                        }
                    }
                }
            }
            s /= 2.0;
            p *= P_RATE;
            i = (I_RATE as f64 * i as f64) as usize;

            self.update_edge_divisions(p, &mut edge_subdivisions);
        }
        edge_subdivisions
    }

    pub fn calculate(&self) -> Vec<Vec<GraphNode>> {
        let mut edge_subdivisions = self.initialize_edge_subdivisions();
        let mut compatibility_lists = self.initialize_compatibility_lists();
        self.update_edge_divisions(P_INITIAL, &mut edge_subdivisions);

        self.compute_compatibility_lists(&mut compatibility_lists);

        self.do_cycles(edge_subdivisions, &compatibility_lists)
    }
}
