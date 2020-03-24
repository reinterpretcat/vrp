//! A command line interface to *Vehicle Routing Problem* solver.
//!
//! For more details please check [docs](cli/index.html)

mod import;
use self::import::get_import_app;
use self::import::run_import;

mod solve;
use self::solve::get_solve_app;
use self::solve::run_solve;

extern crate clap;
use clap::{App, Arg, ArgMatches, Values};
use std::fs::File;
use std::io::{stdout, BufWriter, Write};
use std::process;

fn main() {
    let matches = App::new("Vehicle Routing Problem Solver")
        .version("0.1")
        .author("Ilya Builuk <ilya.builuk@gmail.com>")
        .about("A command line interface to Vehicle Routing Problem solver")
        .subcommand(get_solve_app())
        .subcommand(get_import_app())
        .get_matches();

    match matches.subcommand() {
        ("solve", Some(solve_matches)) => run_solve(solve_matches),
        ("import", Some(import_matches)) => run_import(import_matches),
        ("", None) => eprintln!("No subcommand was used"),
        _ => unreachable!(),
    }
}

fn open_file(path: &str, description: &str) -> File {
    File::open(path).unwrap_or_else(|err| {
        eprintln!("Cannot open {} file '{}': '{}'", description, path, err.to_string());
        process::exit(1);
    })
}

fn create_file(path: &str, description: &str) -> File {
    File::create(path).unwrap_or_else(|err| {
        eprintln!("Cannot create {} file '{}': '{}'", description, path, err.to_string());
        process::exit(1);
    })
}

fn create_write_buffer(out_file: Option<File>) -> BufWriter<Box<dyn Write>> {
    if let Some(out_file) = out_file {
        BufWriter::new(Box::new(out_file))
    } else {
        BufWriter::new(Box::new(stdout()))
    }
}
