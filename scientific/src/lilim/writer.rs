use crate::common::write_text_solution;
use core::models::Solution;
use std::io::{BufWriter, Error, Write};

pub fn write_lilim_solution<W: Write>(writer: BufWriter<W>, solution: &Solution) -> Result<(), Error> {
    write_text_solution(writer, solution)
}
