use super::*;

struct TestAdjacencySpec {
    matrix: [[i32; 13]; 13],
    neighbours: Vec<Vec<Node>>,
}

impl TestAdjacencySpec {
    #[rustfmt::skip]
    fn new() -> Self {
            let matrix = [
                [   0, 2451,  713, 1018, 1631, 1374, 2408,  213, 2571,  875, 1420, 2145, 1972], // New York
                [2451,    0, 1745, 1524,  831, 1240,  959, 2596,  403, 1589, 1374,  357,  579], // Los Angeles
                [ 713, 1745,    0,  355,  920,  803, 1737,  851, 1858,  262,  940, 1453, 1260], // Chicago
                [1018, 1524,  355,    0,  700,  862, 1395, 1123, 1584,  466, 1056, 1280,  987], // Minneapolis
                [1631,  831,  920,  700,    0,  663, 1021, 1769,  949,  796,  879,  586,  371], // Denver
                [1374, 1240,  803,  862,  663,    0, 1681, 1551, 1765,  547,  225,  887,  999], // Dallas
                [2408,  959, 1737, 1395, 1021, 1681,    0, 2493,  678, 1724, 1891, 1114,  701], // Seattle
                [ 213, 2596,  851, 1123, 1769, 1551, 2493,    0, 2699, 1038, 1605, 2300, 2099], // Boston
                [2571,  403, 1858, 1584,  949, 1765,  678, 2699,    0, 1744, 1645,  653,  600], // San Francisco
                [ 875, 1589,  262,  466,  796,  547, 1724, 1038, 1744,    0,  679, 1272, 1162], // St. Louis
                [1420, 1374,  940, 1056,  879,  225, 1891, 1605, 1645,  679,    0, 1017, 1200], // Houston
                [2145,  357, 1453, 1280,  586,  887, 1114, 2300,  653, 1272, 1017,    0,  504], // Phoenix
                [1972,  579, 1260,  987,  371,  999,  701, 2099,  600, 1162,  1200,  504,   0]  // Salt Lake City
            ];

            let neighbours = (0..matrix.len())
            .map(|i| {
                (0..matrix.len())
                    .filter(|&j| j != i)
                    .collect()
            })
            .collect();

            Self { matrix,  neighbours, }
        }
}

impl AdjacencySpec for TestAdjacencySpec {
    fn cost(&self, edge: &Edge) -> Cost {
        self.matrix[edge.0][edge.1] as Cost
    }

    fn neighbours(&self, node: Node) -> &[Node] {
        self.neighbours[node].as_slice()
    }
}

#[test]
fn test_find_closest() {
    let kopt = KOpt::new(TestAdjacencySpec::new());

    let tour = Tour::new(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    let t2i = 12;
    let gain = 1972.;
    let broken = make_edge_set(once((0, 12)));
    let joined = EdgeSet::new();

    let closest = kopt.find_closest(&tour, t2i, gain, &broken, &joined);

    assert_eq!(
        closest,
        vec![
            (6, (1792., 1271.)),
            (1, (1166., 1393.)),
            (8, (1144., 1372.)),
            (5, (682., 973.)),
            (4, (292., 1601.)),
            (10, (-183., 772.)),
            (3, (-287., 985.)),
            (9, (-483., 810.)),
            (2, (-905., 712.)),
        ]
    );
}

#[test]
fn test_choice_x() {
    let kopt = KOpt::new(TestAdjacencySpec::new());
    let tour = Tour::new(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    let t1 = 0;
    let last = 6;
    let gain = 1271.;
    let broken = make_edge_set(once((0, 12)));
    let joined = make_edge_set(once((6, 12)));

    let closest = kopt.choose_x(&tour, t1, last, gain, &broken, &joined).expect("should find path");

    assert_eq!(closest, vec![0, 1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 12, 6]);
}

#[test]
fn test_optimize() {
    let kopt = KOpt::new(TestAdjacencySpec::new());
    let path = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

    let solutions = kopt.optimize(path);

    let best_known = solutions.last().expect("should have solutions");
    assert_eq!(*best_known, vec![0, 7, 2, 3, 4, 12, 6, 8, 1, 11, 10, 5, 9]);
}
