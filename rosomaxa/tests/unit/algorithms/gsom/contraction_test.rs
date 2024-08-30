use super::*;
use crate::helpers::algorithms::gsom::*;

fn insert(coord: (i32, i32), network: &mut Network<Data, DataStorage, DataStorageFactory>) {
    network.insert(coord.into(), &[coord.0 as Float, coord.1 as Float, 0.]);
}

fn assert_network(
    expected: Vec<((i32, i32), (Float, Float))>,
    network: &Network<Data, DataStorage, DataStorageFactory>,
) {
    expected.into_iter().for_each(|(new_coord, old_coord)| {
        let weights = network.find(&new_coord.into()).unwrap().weights.clone();
        assert_eq!(weights, vec![old_coord.0, old_coord.1, 0.])
    });
}

parameterized_test! {can_contract_small_network, (decimation, coords, expected), {
    can_contract_small_network_impl(decimation, coords, expected);
}}

can_contract_small_network! {
        case01_one_row_one_col: ((3, 4),
            vec![(0, 0), (0, 1), (1, 0), (1, 1), (-1, 0), (-2, 0), (0, -1), (0, -2), (1, -1), (1, -2), (2, 1)],
            vec![((2, 1), (2., 1.)), ((1, 1), (1., 1.)), ((1, 0), (1., -1.)), ((1, -1), (1., -2.))]
        ),

        case02_one_row_three_cols: ((2, 3),
            vec![(0, 0), (0, 1), (1, 0), (1, 1), (-1, 0), (-2, 0), (0, -1), (0, -2), (1, -1), (1, -2), (2, 1)],
            vec![((1, -1), (1., -2.)), ((1, 1), (1., 1.)), ((1, 0), (1., -1.))]
        ),
}

fn can_contract_small_network_impl(
    decimation: (i32, i32),
    coords: Vec<(i32, i32)>,
    expected: Vec<((i32, i32), (Float, Float))>,
) {
    let mut network = create_test_network(false);
    coords.into_iter().for_each(|coord| insert(coord, &mut network));

    contract_graph(&mut network, decimation);

    assert_eq!(network.size(), expected.len());
    assert_network(expected, &network);
}

#[test]
fn can_contract_fat_network() {
    let mut network = create_test_network(false);
    for i in -2..=2 {
        for j in -1..=1 {
            insert((i, j), &mut network)
        }
    }

    contract_graph(&mut network, (2, 3));

    assert_network(vec![((0, 0), (-1., -1.)), ((0, 1), (-1., 1.)), ((1, 1), (1., 1.)), ((1, 0), (1., -1.))], &network);
}

#[test]
fn can_get_offset() {
    // right normal
    assert_eq!(get_offset(1, (-100, 10), 2), 0);
    assert_eq!(get_offset(2, (-100, 10), 2), -1);
    assert_eq!(get_offset(3, (-100, 10), 2), -1);
    assert_eq!(get_offset(4, (-100, 10), 2), -2);
    assert_eq!(get_offset(5, (-100, 10), 2), -2);
    assert_eq!(get_offset(6, (-100, 10), 2), -3);

    // right with extra offset
    assert_eq!(get_offset(1, (0, 10), 2), -1);
    assert_eq!(get_offset(3, (0, 10), 2), -2);
    assert_eq!(get_offset(5, (0, 10), 2), -3);
    assert_eq!(get_offset(6, (0, 10), 2), -4);

    // left normal
    assert_eq!(get_offset(-3, (-10, 100), 2), 1);
    assert_eq!(get_offset(-5, (-10, 100), 2), 2);

    // left with extra offset
    assert_eq!(get_offset(-3, (-10, 0), 2), 2);
    assert_eq!(get_offset(-5, (-10, 0), 2), 3);
}
