use core::models::{Problem, Solution};
use here::json::problem::HereProblem;
use scientific::common::read_init_solution;
use scientific::lilim::{write_lilim_solution, LilimProblem};
use scientific::solomon::{write_solomon_solution, SolomonProblem};
use std::collections::HashMap;
use std::fs::File;
use std::io::{stdout, BufReader, BufWriter, Error};
use std::sync::Arc;

pub struct ProblemReader(pub Box<dyn Fn(File, Option<Vec<File>>) -> Result<Problem, String>>);

pub struct InitSolutionReader(pub Box<dyn Fn(File, Arc<Problem>) -> Option<Solution>>);

pub struct SolutionWriter(pub Box<dyn Fn(Solution) -> Result<(), Error>>);

pub fn get_formats<'a>() -> HashMap<&'a str, (ProblemReader, InitSolutionReader, SolutionWriter)> {
    vec![
        (
            "solomon",
            (
                ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    problem.parse_solomon()
                })),
                InitSolutionReader(Box::new(|file, problem| read_init_solution(BufReader::new(file), problem).ok())),
                SolutionWriter(Box::new(|solution: Solution| {
                    write_solomon_solution(BufWriter::new(Box::new(stdout())), &solution)
                })),
            ),
        ),
        (
            "lilim",
            (
                ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    problem.parse_lilim()
                })),
                InitSolutionReader(Box::new(|_file, _problem| None)),
                SolutionWriter(Box::new(|solution: Solution| {
                    write_lilim_solution(BufWriter::new(Box::new(stdout())), &solution)
                })),
            ),
        ),
        (
            "here",
            (
                ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_some());
                    (problem, matrices.unwrap()).read_here()
                })),
                InitSolutionReader(Box::new(|_file, _problem| None)),
                SolutionWriter(Box::new(|_solution: Solution| unimplemented!())),
            ),
        ),
    ]
    .into_iter()
    .collect()
}
