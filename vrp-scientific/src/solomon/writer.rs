use crate::common::write_text_solution;
use std::borrow::Borrow;
use std::io::{BufWriter, Write};
use vrp_core::models::Solution;

/// A trait to write solomon solution.
pub trait SolomonSolution<W: Write> {
    /// Writes solomon solution.
    fn write_solomon(&self, writer: &mut BufWriter<W>) -> Result<(), String>;
}

impl<W: Write, B: Borrow<Solution>> SolomonSolution<W> for B {
    fn write_solomon(&self, writer: &mut BufWriter<W>) -> Result<(), String> {
        write_text_solution(self.borrow(), writer).map_err(|err| err.to_string())?;
        Ok(())
    }
}
