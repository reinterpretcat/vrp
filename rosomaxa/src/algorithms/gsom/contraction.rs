#[cfg(test)]
#[path = "../../../tests/unit/algorithms/gsom/contraction_test.rs"]
mod contraction_test;

use super::*;
use std::cmp::Ordering;

/// Reduces (graph contraction) a network keeping it connected.
/// NOTE: a very naive implementation: we just decimate rows and columns, shifting the rest
///       node coordinates correspondingly. This way we keep graph (network) connected and
///       respect weight distribution.
pub(crate) fn contract_graph<I, S, F>(network: &mut Network<I, S, F>, decimation: (i32, i32))
where
    I: Input,
    S: Storage<Item = I>,
    F: StorageFactory<I, S>,
{
    // determine decimation step
    let (decim_min, decim_max) = decimation;
    let ((x_min, x_max), (y_min, y_max)) = get_network_shape(network);
    let (x_decim, y_decim) = match (x_max - x_min, y_max - y_min) {
        (x, y) if x > y => (decim_min, decim_max),
        (x, y) if x < y => (decim_max, decim_min),
        _ => (decim_max, decim_max),
    };

    // find nodes which should be removed
    let removed = network
        .get_nodes()
        .map(|node| node.coordinate)
        .filter(|coord| coord.0 % x_decim == 0 || coord.1 % y_decim == 0)
        .collect::<Vec<_>>();

    // remove nodes with given coordinates, but keep track of their data
    let data = removed.iter().fold(Vec::new(), |mut data, coordinate| {
        let node = network.get_mut(coordinate).unwrap();
        data.extend(node.storage.drain(0..));
        network.remove(coordinate);

        data
    });

    // detect what was deleted and shift coordinates of all affected nodes to retain connectivity
    // shift not only to the right/top, but also to the left/bottom to keep center around (0, 0)
    network.remap(&|Coordinate(x, y), node| {
        let mut node = node;
        let x = x + get_offset(x, (x_min, x_max), x_decim);
        let y = y + get_offset(y, (y_min, y_max), y_decim);
        node.coordinate = Coordinate(x, y);

        node
    });

    // NOTE: this is unfortunate, probably, compact was called too often on low amount of nodes
    if network.size() == 0 {
        let dimension = network.dimension();
        let get_weights = |idx: usize| -> Vec<Float> {
            data.get(idx).map_or_else(
                // NOTE: probably, very bad initialization..
                || (0..dimension).map(|_| idx as Float).collect(),
                |data| data.weights().to_vec(),
            )
        };

        network.insert((0, 0).into(), get_weights(0).as_slice());
        network.insert((0, 1).into(), get_weights(1).as_slice());
        network.insert((1, 0).into(), get_weights(2).as_slice());
        network.insert((1, 1).into(), get_weights(3).as_slice());
    }

    // reintroduce data from deleted notes to the network while network growth is not allowed.
    network.train_on_data(data, false);
}

fn get_offset(v: i32, min_max: (i32, i32), decim: i32) -> i32 {
    let (left, right) = (min_max.0.abs(), min_max.1.abs());

    let extra = match v.cmp(&0) {
        Ordering::Greater if right > left => -1,
        Ordering::Less if right <= left => 1,
        Ordering::Less | Ordering::Greater => 0,
        _ => unreachable!(),
    };

    -v / decim + extra
}
