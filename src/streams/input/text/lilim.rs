#[cfg(test)]
#[path = "../../../../tests/unit/streams/input/text/solomon_test.rs"]
mod solomon_test;

#[path = "./matrix_factory.rs"]
mod matrix_factory;
use self::matrix_factory::MatrixFactory;

use crate::models::Problem;
use crate::streams::input::text::StringReader;
use std::fs::File;
use std::io::{BufReader, Read};

pub fn read_lilim_format<R: Read>(mut reader: BufReader<R>) -> Result<Problem, String> {
    LilimReader { buffer: String::new(), reader, matrix: MatrixFactory::new() }.read_problem()
}

pub trait LilimProblem {
    fn parse_lilim(&self) -> Result<Problem, String>;
}

impl LilimProblem for File {
    fn parse_lilim(&self) -> Result<Problem, String> {
        read_lilim_format(BufReader::new(self))
    }
}

impl LilimProblem for String {
    fn parse_lilim(&self) -> Result<Problem, String> {
        read_lilim_format(BufReader::new(StringReader::new(self.as_str())))
    }
}

struct LilimReader<R: Read> {
    buffer: String,
    reader: BufReader<R>,
    matrix: MatrixFactory,
}

impl<R: Read> LilimReader<R> {
    pub fn read_problem(&mut self) -> Result<Problem, String> {
        unimplemented!()
    }
}
