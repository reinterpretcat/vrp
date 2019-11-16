use crate::common::write_text_solution;
use core::models::Solution;
use std::io::{BufWriter, Write};

pub trait SolomonSolution<W: Write> {
    fn write_solomon(&self, writer: BufWriter<W>) -> Result<(), String>;
}

impl<W: Write> SolomonSolution<W> for Solution {
    fn write_solomon(&self, writer: BufWriter<W>) -> Result<(), String> {
        write_text_solution(writer, &self).map_err(|err| err.to_string())?;
        Ok(())
    }
}
