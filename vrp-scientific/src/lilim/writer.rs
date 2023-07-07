use crate::common::write_text_solution;
use std::io::{BufWriter, Write};
use vrp_core::models::Solution;

/// A trait to write lilim solution.
pub trait LilimSolution<W: Write> {
    /// Writes lilim solution.
    fn write_lilim(&self, writer: &mut BufWriter<W>) -> Result<(), String>;
}

impl<W: Write> LilimSolution<W> for (&Solution, f64) {
    fn write_lilim(&self, writer: &mut BufWriter<W>) -> Result<(), String> {
        write_text_solution(self.0, self.1, writer).map_err(|err| err.to_string())?;
        Ok(())
    }
}
