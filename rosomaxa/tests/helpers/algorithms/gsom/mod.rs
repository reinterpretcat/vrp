use crate::algorithms::gsom::{Input, Network, NetworkConfig, Storage, StorageFactory};
use crate::algorithms::math::relative_distance;
use crate::utils::{DefaultRandom, Float};
use std::fmt::{Display, Formatter};
use std::ops::RangeBounds;
use std::sync::Arc;

#[derive(Clone)]
pub struct Data {
    pub values: Vec<Float>,
}

impl Input for Data {
    fn weights(&self) -> &[Float] {
        self.values.as_slice()
    }
}

impl Data {
    pub fn new(x: Float, y: Float, z: Float) -> Self {
        Self { values: vec![x, y, z] }
    }
}

#[derive(Default)]
pub struct DataStorage {
    pub data: Vec<Data>,
}

impl Storage for DataStorage {
    type Item = Data;

    fn add(&mut self, input: Self::Item) {
        self.data.clear();
        self.data.push(input);
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Item> + 'a> {
        Box::new(self.data.iter())
    }

    fn drain<R>(&mut self, range: R) -> Vec<Self::Item>
    where
        R: RangeBounds<usize>,
    {
        self.data.drain(range).collect()
    }

    fn distance(&self, a: &[Float], b: &[Float]) -> Float {
        relative_distance(a.iter().cloned(), b.iter().cloned())
    }

    fn size(&self) -> usize {
        self.data.len()
    }
}

impl Display for DataStorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data.len())
    }
}

pub struct DataStorageFactory;

impl StorageFactory<(), Data, DataStorage> for DataStorageFactory {
    fn eval(&self, _: &()) -> DataStorage {
        DataStorage::default()
    }
}

pub fn create_test_network(has_initial_error: bool) -> Network<(), Data, DataStorage, DataStorageFactory> {
    Network::new(
        &(),
        [
            Data::new(0.230529, 0.956665, 0.482008),
            Data::new(0.400775, 0.142917, 0.555519),
            Data::new(0.260272, 0.175342, 0.193711),
            Data::new(0.186712, 0.166380, 0.773621),
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
