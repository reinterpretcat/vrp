use crate::common::write_text_solution;
use std::io::{BufWriter, Write};
use vrp_core::models::Solution;

/// A trait to write solomon solution.
pub trait SolomonSolution<W: Write> {
    /// Writes solomon solution.
    fn write_solomon(&self, writer: &mut BufWriter<W>) -> Result<(), String>;
}

impl<W: Write> SolomonSolution<W> for (&Solution, f64) {
    fn write_solomon(&self, writer: &mut BufWriter<W>) -> Result<(), String> {
        write_text_solution(self.0, self.1, writer).map_err(|err| err.to_string())?;
        Ok(())
    }
}
