use super::*;
use hashbrown::{HashMap, HashSet};
use std::cmp::Ordering::Less;

/// A simple `AdjacencyMatrix` using naive sparse matrix implementation.
pub struct SparseMatrix {
    pub data: HashMap<usize, Vec<(usize, f64)>>,
    pub values: HashSet<i64>,
    pub size: usize,
}

impl AdjacencyMatrix for SparseMatrix {
    fn new(size: usize) -> Self {
        Self { data: Default::default(), values: Default::default(), size }
    }

    fn values<'a>(&'a self) -> Box<dyn Iterator<Item = f64> + 'a> {
        Box::new(self.values.iter().map(|&v| unsafe { std::mem::transmute(v) }))
    }

    fn set_cell(&mut self, row: usize, col: usize, value: f64) {
        let cells = self.data.entry(row).or_insert_with(|| vec![]);
        cells.push((col, value));
        cells.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(Less));
        self.values.insert(unsafe { std::mem::transmute(value) });
    }

    fn scan_row<F>(&self, row: usize, predicate: F) -> Option<usize>
    where
        F: Fn(f64) -> bool,
    {
        self.data.get(&row).and_then(|cells| cells.iter().find(|(_, v)| predicate(*v))).map(|(col, _)| *col)
    }
}
