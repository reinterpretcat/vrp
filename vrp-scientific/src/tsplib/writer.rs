use crate::common::write_text_solution;
use std::borrow::Borrow;
use std::io::{BufWriter, Write};
use vrp_core::prelude::*;

/// A trait to write tsplib95 solution.
pub trait TsplibSolution<W: Write> {
    /// Writes tsplib95 solution.
    fn write_tsplib(&self, writer: &mut BufWriter<W>) -> Result<(), GenericError>;
}

impl<W: Write, B: Borrow<Solution>> TsplibSolution<W> for B {
    fn write_tsplib(&self, writer: &mut BufWriter<W>) -> Result<(), GenericError> {
        write_text_solution(self.borrow(), writer).map_err(|err| err.to_string())?;
        Ok(())
    }
}
