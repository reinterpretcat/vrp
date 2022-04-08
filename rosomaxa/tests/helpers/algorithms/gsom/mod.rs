use crate::algorithms::gsom::{Input, Network, NetworkConfig, Storage, StorageFactory};
use crate::utils::DefaultRandom;
use std::fmt::{Display, Formatter};
use std::ops::RangeBounds;
use std::sync::Arc;

#[derive(Clone)]
pub struct Data {
    pub values: Vec<f64>,
}

impl Input for Data {
    fn weights(&self) -> &[f64] {
        self.values.as_slice()
    }
}

impl Data {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { values: vec![x, y, z] }
    }
}

pub struct DataStorage {
    pub data: Vec<Data>,
}

impl Storage for DataStorage {
    type Item = Data;

    fn add(&mut self, input: Self::Item) {
        self.data.clear();
        self.data.push(input);
    }

    fn drain<R>(&mut self, range: R) -> Vec<Self::Item>
    where
        R: RangeBounds<usize>,
    {
        self.data.drain(range).collect()
    }

    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        f64::sqrt((a[0] - b[0]).powf(2.0) + (a[1] - b[1]).powf(2.0) + (a[2] - b[2]).powf(2.0))
    }

    fn size(&self) -> usize {
        self.data.len()
    }
}

impl Default for DataStorage {
    fn default() -> Self {
        Self { data: Default::default() }
    }
}

impl Display for DataStorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data.len())
    }
}

pub struct DataStorageFactory;

impl StorageFactory<Data, DataStorage> for DataStorageFactory {
    fn eval(&self) -> DataStorage {
        DataStorage::default()
    }
}

pub fn create_test_network(has_initial_error: bool) -> Network<Data, DataStorage, DataStorageFactory> {
    Network::new(
        [
            Data::new(0.23052992, 0.95666552, 0.48200831),
            Data::new(0.40077599, 0.14291798, 0.55551944),
            Data::new(0.26027299, 0.17534256, 0.19371101),
            Data::new(0.18671211, 0.16638008, 0.77362103),
        ],
        NetworkConfig {
            spread_factor: 0.25,
            distribution_factor: 0.25,
            learning_rate: 0.1,
            rebalance_memory: 100,
            has_initial_error,
        },
        Arc::new(DefaultRandom::default()),
        DataStorageFactory,
    )
}
