use core::models::{Problem, Solution};
use here::json::problem::HereProblem;
use here::json::solution::HereSolution;
use scientific::common::read_init_solution;
use scientific::lilim::{LilimProblem, LilimSolution};
use scientific::solomon::{SolomonProblem, SolomonSolution};
use std::collections::HashMap;
use std::fs::File;
use std::io::{stdout, BufReader, BufWriter};
use std::sync::Arc;

pub struct ProblemReader(pub Box<dyn Fn(File, Option<Vec<File>>) -> Result<Problem, String>>);

pub struct InitSolutionReader(pub Box<dyn Fn(File, Arc<Problem>) -> Option<Solution>>);

pub struct SolutionWriter(pub Box<dyn Fn(&Problem, Solution) -> Result<(), String>>);

pub fn get_formats<'a>() -> HashMap<&'a str, (ProblemReader, InitSolutionReader, SolutionWriter)> {
    vec![
        (
            "solomon",
            (
                ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    problem.read_solomon()
                })),
                InitSolutionReader(Box::new(|file, problem| read_init_solution(BufReader::new(file), problem).ok())),
                SolutionWriter(Box::new(|_, solution| solution.write_solomon(BufWriter::new(Box::new(stdout()))))),
            ),
        ),
        (
            "lilim",
            (
                ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    problem.read_lilim()
                })),
                InitSolutionReader(Box::new(|_file, _problem| None)),
                SolutionWriter(Box::new(|_, solution| solution.write_lilim(BufWriter::new(Box::new(stdout()))))),
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
                SolutionWriter(Box::new(|problem, solution| {
                    solution.write_here(problem, BufWriter::new(Box::new(stdout())))
                })),
            ),
        ),
    ]
    .into_iter()
    .collect()
}
