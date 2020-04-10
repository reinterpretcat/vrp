//! A command line interface to *Vehicle Routing Problem* solver.
//!

use clap::App;
use std::process;
use vrp_cli::import::{get_import_app, run_import};
use vrp_cli::solve::{get_solve_app, run_solve};

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
        ("", None) => {
            eprintln!("No subcommand was used. Use -h to print help information.");
            process::exit(1);
        }
        _ => unreachable!(),
    }
}
