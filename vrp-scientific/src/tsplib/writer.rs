use crate::common::write_text_solution;
use std::io::{BufWriter, Write};
use vrp_core::models::Solution;

/// A trait to write tsplib95 solution.
pub trait TsplibSolution<W: Write> {
    /// Writes tsplib95 solution.
    fn write_tsplib(&self, writer: BufWriter<W>) -> Result<(), String>;
}

impl<W: Write> TsplibSolution<W> for Solution {
    fn write_tsplib(&self, writer: BufWriter<W>) -> Result<(), String> {
        write_text_solution(writer, self).map_err(|err| err.to_string())?;
        Ok(())
    }
}
