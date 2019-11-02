use core::models::Problem;
use std::fs::File;
use std::io::BufReader;

#[path = "./deserializer.rs"]
mod deserializer;
use self::deserializer::*;
type ApiProblem = self::deserializer::Problem;

#[path = "./utils.rs"]
mod utils;
use self::utils::*;

pub trait HereProblem {
    fn parse_here(&self) -> Result<Problem, String>;
}

impl HereProblem for (File, Vec<File>) {
    fn parse_here(&self) -> Result<Problem, String> {
        let problem = deserialize_problem(BufReader::new(&self.0)).map_err(|err| err.to_string())?;

        let matrices = self.1.iter().fold(vec![], |mut acc, matrix| {
            acc.push(deserialize_matrix(BufReader::new(matrix)).unwrap());
            acc
        });

        map_to_problem(problem, matrices)
    }
}

impl HereProblem for (String, Vec<String>) {
    fn parse_here(&self) -> Result<Problem, String> {
        let problem = deserialize_problem(BufReader::new(StringReader::new(&self.0))).map_err(|err| err.to_string())?;

        let matrices = self.1.iter().fold(vec![], |mut acc, matrix| {
            acc.push(deserialize_matrix(BufReader::new(StringReader::new(matrix))).unwrap());
            acc
        });

        map_to_problem(problem, matrices)
    }
}

fn map_to_problem(api_problem: ApiProblem, matrices: Vec<Matrix>) -> Result<Problem, String> {
    unimplemented!()
}
