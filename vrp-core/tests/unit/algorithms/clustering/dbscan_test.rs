use super::*;
use crate::algorithms::geometry::Point;
use crate::helpers::construction::clustering::p;
use rosomaxa::prelude::compare_floats;

fn create_index(points: &[Point]) -> HashMap<&Point, Vec<(&Point, Float)>> {
    points.iter().fold(HashMap::new(), |mut acc, point| {
        assert!(!acc.contains_key(point));

        let mut pairs = points
            .iter()
            .filter(|other| *other != point)
            .map(|other| (other, point.distance_to_point(other)))
            .collect::<Vec<_>>();

        pairs.sort_by(|(_, a), (_, b)| compare_floats(*a, *b));

        acc.insert(point, pairs);

        acc
    })
}

fn create_neighborhood<'a>(index: &'a HashMap<&'a Point, Vec<(&'a Point, Float)>>) -> NeighborhoodFn<'a, Point> {
    Box::new(move |item: &Point, eps: Float| {
        Box::new(
            index.get(item).unwrap().iter().take_while(move |(_, distance)| *distance < eps).map(|(point, _)| *point),
        )
    })
}

fn assert_non_ordered(actual: Vec<&Point>, expected: Vec<&Point>) {
    let mut actual = actual;
    let mut expected = expected;

    actual.sort_by(|a, b| compare_floats(a.x, b.x));
    expected.sort_by(|a, b| compare_floats(a.x, b.x));

    assert_eq!(actual, expected);
}

// NOTE test data is borrowed from https://github.com/apache/commons-math

#[test]
fn can_create_clusters_normally() {
    let ps = vec![
        p(83.08303, 58.83387),
        p(45.05445, 23.46964),
        p(14.96417, 69.0264),
        p(73.53189, 34.89614),
        p(73.28498, 33.9686),
        p(73.45828, 33.92584),
        p(73.96578, 35.73191),
        p(74.0074, 36.81735),
        p(73.41247, 34.27314),
        p(73.91562, 36.83206),
        p(74.81499, 37.15682),
        p(74.03144, 37.57399),
        p(74.5187, 38.67425),
        p(74.50754, 35.58903),
        p(74.51322, 36.03057),
        p(59.279, 46.41091),
        p(59.73744, 46.20015),
        p(58.81134, 45.7115),
        p(58.52225, 47.41608),
        p(58.21862, 47.36228),
        p(60.27139, 46.6061),
        p(60.89496, 46.97692),
        p(62.29048, 47.6697),
        p(61.03857, 46.21292),
        p(60.16916, 45.18193),
        p(59.90036, 47.55536),
        p(62.33003, 47.83941),
        p(57.86035, 47.31117),
        p(58.13715, 48.98596),
        p(56.13192, 46.85089),
        p(55.97632, 47.46384),
        p(56.23245, 47.94003),
        p(58.51687, 46.62288),
        p(57.85411, 45.95394),
        p(56.44577, 45.16209),
        p(57.36691, 47.50097),
        p(58.24362, 46.11405),
        p(56.27224, 44.79908),
        p(57.60692, 46.94291),
        p(30.18714, 13.87714),
        p(30.44944, 13.49077),
        p(30.29501, 13.26488),
        p(30.1602, 11.89278),
        p(31.3415, 15.28265),
        p(31.68601, 14.75687),
        p(29.32596, 12.09784),
        p(29.5482, 13.61329),
        p(28.79359, 10.36352),
        p(31.01284, 12.78847),
        p(27.58509, 11.4757),
        p(28.59379, 10.78099),
        p(31.3561, 15.08031),
        p(31.25948, 13.67432),
        p(32.3159, 14.95261),
        p(30.46041, 15.88402),
        p(32.56178, 14.58607),
        p(32.76138, 16.23983),
        p(30.18294, 14.70959),
        p(29.55088, 15.06512),
        p(29.00415, 14.08966),
        p(29.33962, 13.29096),
        p(30.99746, 14.55191),
        p(30.66784, 16.2697),
    ];
    let index = create_index(&ps);
    let neighborhood_fn = create_neighborhood(&index);

    let clusters = create_clusters(ps.as_slice(), 2., 5, &neighborhood_fn);

    assert_eq!(clusters.len(), 3);

    assert_non_ordered(
        clusters[0].clone(),
        vec![&ps[3], &ps[4], &ps[5], &ps[6], &ps[7], &ps[8], &ps[9], &ps[10], &ps[11], &ps[12], &ps[13], &ps[14]],
    );
    assert_non_ordered(
        clusters[1].clone(),
        vec![
            &ps[15], &ps[16], &ps[17], &ps[18], &ps[19], &ps[20], &ps[21], &ps[22], &ps[23], &ps[24], &ps[25], &ps[26],
            &ps[27], &ps[28], &ps[29], &ps[30], &ps[31], &ps[32], &ps[33], &ps[34], &ps[35], &ps[36], &ps[37], &ps[38],
        ],
    );
    assert_non_ordered(
        clusters[2].clone(),
        vec![
            &ps[39], &ps[40], &ps[41], &ps[42], &ps[43], &ps[44], &ps[45], &ps[46], &ps[47], &ps[48], &ps[49], &ps[50],
            &ps[51], &ps[52], &ps[53], &ps[54], &ps[55], &ps[56], &ps[57], &ps[58], &ps[59], &ps[60], &ps[61], &ps[62],
        ],
    );
}

#[test]
fn can_create_clusters_with_single_link() {
    let ps = vec![
        p(10., 10.), // A
        p(12., 9.),
        p(10., 8.),
        p(8., 8.),
        p(8., 6.),
        p(7., 7.),
        p(5., 6.),  // B
        p(14., 8.), // C
        p(7., 15.), // N - noise, should not be present
        p(17., 8.), // D - single-link connected to C should not be present
    ];
    let index = create_index(&ps);
    let neighborhood_fn = create_neighborhood(&index);

    let clusters = create_clusters(ps.as_slice(), 3., 3, &neighborhood_fn);

    assert_eq!(clusters.len(), 1);

    assert_non_ordered(clusters[0].clone(), vec![&ps[0], &ps[1], &ps[2], &ps[3], &ps[4], &ps[5], &ps[6], &ps[7]]);
}
