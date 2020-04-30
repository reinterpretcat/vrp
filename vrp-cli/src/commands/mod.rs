use clap::{App, Arg, ArgMatches, Values};

pub mod check;
pub mod generate;
pub mod import;
pub mod solve;

use std::fs::File;
use std::io::{stdout, BufWriter, Write};
use std::process;

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
