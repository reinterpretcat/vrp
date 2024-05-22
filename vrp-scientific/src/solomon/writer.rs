use crate::common::write_text_solution;
use std::borrow::Borrow;
use std::io::{BufWriter, Write};
use vrp_core::prelude::*;

/// A trait to write solomon solution.
pub trait SolomonSolution<W: Write> {
    /// Writes solomon solution.
    fn write_solomon(&self, writer: &mut BufWriter<W>) -> Result<(), GenericError>;
}

impl<W: Write, B: Borrow<Solution>> SolomonSolution<W> for B {
    fn write_solomon(&self, writer: &mut BufWriter<W>) -> Result<(), GenericError> {
        write_text_solution(self.borrow(), writer).map_err(From::from)
    }
}
