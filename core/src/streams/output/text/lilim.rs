use crate::models::Solution;
use crate::streams::output::text::write_text_solution;
use std::io::{BufWriter, Error, Write};

pub fn write_lilim_solution<W: Write>(writer: BufWriter<W>, solution: &Solution) -> Result<(), Error> {
    write_text_solution(writer, solution)
}
