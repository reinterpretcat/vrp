use crate::common::write_text_solution;
use core::models::Solution;
use std::io::{BufWriter, Write};

pub trait LilimSolution<W: Write> {
    fn write_lilim(&self, writer: BufWriter<W>) -> Result<(), String>;
}

impl<W: Write> LilimSolution<W> for Solution {
    fn write_lilim(&self, writer: BufWriter<W>) -> Result<(), String> {
        write_text_solution(writer, &self).map_err(|err| err.to_string())?;
        Ok(())
    }
}
