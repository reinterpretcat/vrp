//! A tweaked version of `BitVec` from `probabilistic-collections` crate.
use std::fmt::Display;
use std::ops::Index;

#[doc(hidden)]
#[derive(Clone, Debug, PartialEq)]
pub struct BitVec {
    blocks: Vec<u8>,
    length: usize,
}

const BITS_IN_BLOCK: usize = std::mem::size_of::<u8>() * 8;

impl BitVec {
    pub fn new(length: usize) -> Self {
        let block_count = length.div_ceil(BITS_IN_BLOCK);
        Self { blocks: vec![0; block_count], length }
    }

    pub fn set(&mut self, index: usize, bit: bool) {
        assert!(index < self.length);
        let block_index = index / BITS_IN_BLOCK;
        let bit_index = index % BITS_IN_BLOCK;
        let mask = 1 << bit_index;

        if bit {
            self.blocks[block_index] |= mask;
        } else {
            self.blocks[block_index] &= !mask;
        }
    }

    pub fn get(&self, index: usize) -> Option<bool> {
        self.blocks.get(index / BITS_IN_BLOCK).map(|block| ((block >> (index % BITS_IN_BLOCK)) & 1) != 0)
    }

    pub fn union(&mut self, other: &Self) {
        self.apply(other, |x, y| x | y)
    }

    /*    pub fn intersection(&mut self, other: &Self) {
        self.apply(other, |x, y| x & y)
    }

    pub fn difference(&mut self, other: &Self) {
        self.apply(other, |x, y| x & !y)
    }*/

    fn apply<F>(&mut self, other: &BitVec, mut op: F)
    where
        F: FnMut(u8, u8) -> u8,
    {
        assert_eq!(self.length, other.length, "bit vectors must have the same length");

        self.blocks.iter_mut().zip(other.blocks.iter()).for_each(|(x, y)| {
            *x = op(*x, *y);
        });
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

impl Index<usize> for BitVec {
    type Output = bool;

    fn index(&self, index: usize) -> &bool {
        if self.get(index).expect("index out of bounds.") {
            &true
        } else {
            &false
        }
    }
}

impl Display for BitVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for i in 0..self.length {
            write!(f, "{}", if self.get(i).expect("indexing is messed up") { 1 } else { 0 })?;
        }
        write!(f, "]")
    }
}
