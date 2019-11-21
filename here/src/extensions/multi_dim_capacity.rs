#[cfg(test)]
#[path = "../../tests/unit/extensions/multi_dim_capacity_test.rs"]
mod multi_dim_capacity_test;

use std::cmp::Ordering;
use std::ops::{Add, Sub};

/// Specifies multi dimensional capacity type.
#[derive(Clone, Eq, PartialEq, PartialOrd)]
struct MultiDimensionalCapacity {
    pub capacity: Vec<i32>,
}

impl MultiDimensionalCapacity {
    pub fn from_vec(capacity: Vec<i32>) -> Self {
        Self { capacity }
    }

    fn get(&self, idx: usize) -> i32 {
        *self.capacity.get(idx).unwrap_or(&0)
    }
}

impl Add for MultiDimensionalCapacity {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        fn sum(acc: MultiDimensionalCapacity, rhs: &MultiDimensionalCapacity) -> MultiDimensionalCapacity {
            assert!(acc.capacity.len() >= rhs.capacity.len());

            let mut dimens = acc;

            for (idx, value) in rhs.capacity.iter().enumerate() {
                dimens.capacity[idx] += value;
            }

            dimens
        }

        if self.capacity.len() >= rhs.capacity.len() {
            sum(self, &rhs)
        } else {
            sum(rhs, &self)
        }
    }
}

impl Sub for MultiDimensionalCapacity {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut dimens = if self.capacity.len() >= rhs.capacity.len() {
            self
        } else {
            let mut dimens = self;
            dimens.capacity.resize(rhs.capacity.len(), 0);

            dimens
        };

        for (idx, value) in rhs.capacity.iter().enumerate() {
            dimens.capacity[idx] -= value;
        }

        dimens
    }
}

impl Ord for MultiDimensionalCapacity {
    fn cmp(&self, other: &Self) -> Ordering {
        unimplemented!()
    }
}
