use super::*;

fn get_start() -> Path {
    vec![0, 2, 1, 3]
}

fn edge_set(edges: &[(usize, usize)]) -> EdgeSet {
    edges.into_iter().copied().collect()
}

#[test]
fn test_tour_creation() {
    let t = Tour::new(get_start());

    // NOTE edges always reordered
    assert_eq!(t.edges, edge_set(&[(0, 2), (1, 2), (1, 3), (0, 3)]));
    assert_eq!(t.len(), 4);
}

#[test]
fn test_optimal_2opt() {
    let t = Tour::new([0, 2, 1, 3]);
    let broken = edge_set(&[(0, 2), (1, 3)]);
    let joined = edge_set(&[(0, 1), (2, 3)]);

    let new_tour = t.try_path(&broken, &joined).expect("2-opt failed");

    assert_eq!(new_tour, [0, 1, 2, 3]);
}

#[test]
fn test_optimal_3opt() {
    let t = Tour::new([0, 3, 2, 4, 5, 1]);
    let broken = edge_set(&[(0, 3), (2, 4), (1, 5)]);
    let joined = edge_set(&[(0, 5), (3, 4), (1, 2)]);

    let new_tour = t.try_path(&broken, &joined).expect("3-opt failed");

    assert_eq!(new_tour, [0, 1, 2, 3, 4, 5]);
}

#[test]
fn test_disjoint_path() {
    let t = Tour::new([0, 3, 2, 4, 5, 1]);
    let broken = edge_set(&[(0, 3), (4, 5)]);
    let joined = edge_set(&[(0, 5), (3, 4)]);

    let new_tour = t.try_path(&broken, &joined);

    assert!(new_tour.is_none());
}
