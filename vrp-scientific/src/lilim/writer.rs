use crate::common::write_text_solution;
use std::borrow::Borrow;
use std::io::{BufWriter, Write};
use vrp_core::models::Solution;

/// A trait to write lilim solution.
pub trait LilimSolution<W: Write> {
    /// Writes lilim solution.
    fn write_lilim(&self, writer: &mut BufWriter<W>) -> Result<(), String>;
}

impl<W: Write, B: Borrow<Solution>> LilimSolution<W> for B {
    fn write_lilim(&self, writer: &mut BufWriter<W>) -> Result<(), String> {
        write_text_solution(self.borrow(), writer).map_err(|err| err.to_string())?;
        Ok(())
    }
}
