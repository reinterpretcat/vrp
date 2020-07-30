#[cfg(test)]
#[path = "../../../tests/unit/models/domain/capacity_test.rs"]
mod capacity_test;

use crate::models::common::{Dimensions, ValueDimension};
use std::cmp::Ordering;
use std::iter::Sum;
use std::ops::{Add, Mul, Sub};

const CAPACITY_DIMENSION_KEY: &str = "cpc";
const DEMAND_DIMENSION_KEY: &str = "dmd";
const CAPACITY_DIMENSION_SIZE: usize = 8;

/// Represents a vehicle load type.
pub trait Capacity: Add + Sub + Ord + Copy + Default + Send + Sync {
    /// Returns true if capacity is not an empty.
    fn is_not_empty(&self) -> bool;

    /// Returns max capacity value.
    fn max_load(self, other: Self) -> Self;

    /// Returns true if `other` can be loaded into existing capacity.
    fn can_load(&self, other: &Self) -> bool;
}

/// Represents job demand, both static and dynamic.
pub struct Demand<T: Capacity + Add<Output = T> + Sub<Output = T> + 'static> {
    /// Keeps static and dynamic pickup amount.
    pub pickup: (T, T),
    /// Keeps static and dynamic delivery amount.
    pub delivery: (T, T),
}

/// A trait to get or set capacity.
pub trait CapacityDimension<T: Capacity + Add<Output = T> + Sub<Output = T> + 'static> {
    /// Sets capacity.
    fn set_capacity(&mut self, demand: T) -> &mut Self;
    /// Gets capacity.
    fn get_capacity(&self) -> Option<&T>;
}

/// A trait to get or set demand.
pub trait DemandDimension<T: Capacity + Add<Output = T> + Sub<Output = T> + 'static> {
    /// Sets demand.
    fn set_demand(&mut self, demand: Demand<T>) -> &mut Self;
    /// Gets demand.
    fn get_demand(&self) -> Option<&Demand<T>>;
}

impl<T: Capacity + Add<Output = T> + Sub<Output = T> + 'static> Demand<T> {
    /// Returns capacity change as difference between pickup and delivery.
    pub fn change(&self) -> T {
        self.pickup.0 + self.pickup.1 - self.delivery.0 - self.delivery.1
    }
}

impl<T: Capacity + Add<Output = T> + Sub<Output = T> + 'static> Default for Demand<T> {
    fn default() -> Self {
        Self { pickup: (Default::default(), Default::default()), delivery: (Default::default(), Default::default()) }
    }
}

impl<T: Capacity + Add<Output = T> + Sub<Output = T> + 'static> Clone for Demand<T> {
    fn clone(&self) -> Self {
        Self { pickup: self.pickup, delivery: self.delivery }
    }
}

impl<T: Capacity + Add<Output = T> + Sub<Output = T> + 'static> CapacityDimension<T> for Dimensions {
    fn set_capacity(&mut self, demand: T) -> &mut Self {
        self.set_value(CAPACITY_DIMENSION_KEY, demand);
        self
    }

    fn get_capacity(&self) -> Option<&T> {
        self.get_value(CAPACITY_DIMENSION_KEY)
    }
}

impl<T: Capacity + Add<Output = T> + Sub<Output = T> + 'static> DemandDimension<T> for Dimensions {
    fn set_demand(&mut self, demand: Demand<T>) -> &mut Self {
        self.set_value(DEMAND_DIMENSION_KEY, demand);
        self
    }

    fn get_demand(&self) -> Option<&Demand<T>> {
        self.get_value(DEMAND_DIMENSION_KEY)
    }
}

/// Specifies single dimensional capacity type.
#[derive(Clone, Copy, Debug)]
pub struct SingleDimCapacity {
    /// An actual capacity value.
    pub value: i32,
}

impl SingleDimCapacity {
    /// Creates a new instance of `SingleDimCapacity`.
    pub fn new(value: i32) -> Self {
        Self { value }
    }
}

impl Default for SingleDimCapacity {
    fn default() -> Self {
        Self { value: 0 }
    }
}

impl Capacity for SingleDimCapacity {
    fn is_not_empty(&self) -> bool {
        self.value != 0
    }

    fn max_load(self, other: Self) -> Self {
        let value = self.value.max(other.value);
        Self { value }
    }

    fn can_load(&self, other: &Self) -> bool {
        self.value >= other.value
    }
}

impl Add for SingleDimCapacity {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let value = self.value + rhs.value;
        Self { value }
    }
}

impl Sub for SingleDimCapacity {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let value = self.value - rhs.value;
        Self { value }
    }
}

impl Ord for SingleDimCapacity {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl PartialOrd for SingleDimCapacity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for SingleDimCapacity {}

impl PartialEq for SingleDimCapacity {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Mul<f64> for SingleDimCapacity {
    type Output = Self;

    fn mul(self, value: f64) -> Self::Output {
        Self::new((self.value as f64 * value).round() as i32)
    }
}

/// Specifies multi dimensional capacity type.
#[derive(Clone, Copy, Debug)]
pub struct MultiDimCapacity {
    /// Capacity data.
    pub capacity: [i32; CAPACITY_DIMENSION_SIZE],
    /// Actual used size.
    pub size: usize,
}

impl MultiDimCapacity {
    /// Creates a new instance of `MultiDimCapacity`.
    pub fn new(data: Vec<i32>) -> Self {
        assert!(data.len() <= CAPACITY_DIMENSION_SIZE);

        let mut capacity = [0; CAPACITY_DIMENSION_SIZE];
        for (idx, value) in data.iter().enumerate() {
            capacity[idx] = *value;
        }

        Self { capacity, size: data.len() }
    }

    fn get(&self, idx: usize) -> i32 {
        self.capacity[idx]
    }

    /// Converts to vector representation.
    pub fn as_vec(&self) -> Vec<i32> {
        if self.size == 0 {
            vec![0]
        } else {
            self.capacity[..self.size].to_vec()
        }
    }
}

impl Capacity for MultiDimCapacity {
    fn is_not_empty(&self) -> bool {
        self.size == 0 || self.capacity.iter().any(|v| *v != 0)
    }

    fn max_load(self, other: Self) -> Self {
        let mut result = self;
        result.capacity.iter_mut().zip(other.capacity.iter()).for_each(|(a, b)| *a = (*a).max(*b));

        result
    }

    fn can_load(&self, other: &Self) -> bool {
        self.capacity.iter().zip(other.capacity.iter()).all(|(a, b)| a >= b)
    }
}

impl Default for MultiDimCapacity {
    fn default() -> Self {
        Self { capacity: [0; CAPACITY_DIMENSION_SIZE], size: 0 }
    }
}

impl Add for MultiDimCapacity {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        fn sum(acc: MultiDimCapacity, rhs: &MultiDimCapacity) -> MultiDimCapacity {
            let mut dimens = acc;

            for (idx, value) in rhs.capacity.iter().enumerate() {
                dimens.capacity[idx] += *value;
            }

            dimens.size = dimens.size.max(rhs.size);

            dimens
        }

        if self.capacity.len() >= rhs.capacity.len() {
            sum(self, &rhs)
        } else {
            sum(rhs, &self)
        }
    }
}

impl Sub for MultiDimCapacity {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut dimens = self;

        for (idx, value) in rhs.capacity.iter().enumerate() {
            dimens.capacity[idx] -= *value;
        }

        dimens.size = dimens.size.max(rhs.size);

        dimens
    }
}

impl Ord for MultiDimCapacity {
    fn cmp(&self, other: &Self) -> Ordering {
        let size = self.capacity.len().max(other.capacity.len());
        (0..size).fold(Ordering::Equal, |acc, idx| match acc {
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => self.get(idx).cmp(&other.get(idx)),
            Ordering::Less => {
                if self.get(idx) > other.get(idx) {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
        })
    }
}

impl PartialOrd for MultiDimCapacity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for MultiDimCapacity {}

impl PartialEq for MultiDimCapacity {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Mul<f64> for MultiDimCapacity {
    type Output = Self;

    fn mul(self, value: f64) -> Self::Output {
        let mut dimens = self;

        dimens.capacity.iter_mut().for_each(|item| {
            *item = (*item as f64 * value).round() as i32;
        });

        dimens
    }
}

impl Sum for MultiDimCapacity {
    fn sum<I: Iterator<Item = MultiDimCapacity>>(iter: I) -> Self {
        iter.fold(MultiDimCapacity::default(), |acc, item| item + acc)
    }
}
