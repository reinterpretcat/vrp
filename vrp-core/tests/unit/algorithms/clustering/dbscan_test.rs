use super::*;
use crate::algorithms::geometry::Point;
use crate::helpers::construction::clustering::p;
use rosomaxa::prelude::compare_floats;

fn create_index(points: &[Point]) -> HashMap<&Point, Vec<(&Point, f64)>> {
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

fn create_neighborhood<'a>(index: &'a HashMap<&'a Point, Vec<(&'a Point, f64)>>) -> NeighborhoodFn<'a, Point> {
    Box::new(move |item: &Point, eps: f64| {
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
        p(83.08303244924173, 58.83387754182331),
        p(45.05445510940626, 23.469642649637535),
        p(14.96417921432294, 69.0264096390456),
        p(73.53189604333602, 34.896145021310076),
        p(73.28498173551634, 33.96860806993209),
        p(73.45828098873608, 33.92584423092194),
        p(73.9657889183145, 35.73191006924026),
        p(74.0074097183533, 36.81735596177168),
        p(73.41247541410848, 34.27314856695011),
        p(73.9156256353017, 36.83206791547127),
        p(74.81499205809087, 37.15682749846019),
        p(74.03144880081527, 37.57399178552441),
        p(74.51870941207744, 38.674258946906775),
        p(74.50754595105536, 35.58903978415765),
        p(74.51322752749547, 36.030572259100154),
        p(59.27900996617973, 46.41091720294207),
        p(59.73744793841615, 46.20015558367595),
        p(58.81134076672606, 45.71150126331486),
        p(58.52225539437495, 47.416083617601544),
        p(58.218626647023484, 47.36228902172297),
        p(60.27139669447206, 46.606106348801404),
        p(60.894962462363765, 46.976924697402865),
        p(62.29048673878424, 47.66970563563518),
        p(61.03857608977705, 46.212924720020965),
        p(60.16916214139201, 45.18193661351688),
        p(59.90036905976012, 47.555364347063005),
        p(62.33003634144552, 47.83941489877179),
        p(57.86035536718555, 47.31117930193432),
        p(58.13715479685925, 48.985960494028404),
        p(56.131923963548616, 46.8508904252667),
        p(55.976329887053, 47.46384037658572),
        p(56.23245975235477, 47.940035191131756),
        p(58.51687048212625, 46.622885352699086),
        p(57.85411081905477, 45.95394361577928),
        p(56.445776311447844, 45.162093662656844),
        p(57.36691949656233, 47.50097194337286),
        p(58.243626387557015, 46.114052729681134),
        p(56.27224595635198, 44.799080066150054),
        p(57.606924816500396, 46.94291057763621),
        p(30.18714230041951, 13.877149710431695),
        p(30.449448810657486, 13.490778346545994),
        p(30.295018390286714, 13.264889000216499),
        p(30.160201832884923, 11.89278262341395),
        p(31.341509791789576, 15.282655921997502),
        p(31.68601630325429, 14.756873246748),
        p(29.325963742565364, 12.097849250072613),
        p(29.54820742388256, 13.613295356975868),
        p(28.79359608888626, 10.36352064087987),
        p(31.01284597092308, 12.788479208014905),
        p(27.58509216737002, 11.47570110601373),
        p(28.593799561727792, 10.780998203903437),
        p(31.356105766724795, 15.080316198524088),
        p(31.25948503636755, 13.674329151166603),
        p(32.31590076372959, 14.95261758659035),
        p(30.460413702763617, 15.88402809202671),
        p(32.56178203062154, 14.586076852632686),
        p(32.76138648530468, 16.239837325178087),
        p(30.1829453331884, 14.709592407103628),
        p(29.55088173528202, 15.0651247180067),
        p(29.004155302187428, 14.089665298582986),
        p(29.339624439831823, 13.29096065578051),
        p(30.997460327576846, 14.551914158277214),
        p(30.66784126125276, 16.269703107886016),
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
