use core::models::common::IdDimension;
use core::models::problem::Job;
use core::models::Solution;
use std::io::{BufWriter, Error, ErrorKind, Write};

pub fn write_text_solution<W: Write>(writer: BufWriter<W>, solution: &Solution) -> Result<(), Error> {
    let mut writer = writer;

    if !solution.unassigned.is_empty() {
        return Err(Error::new(ErrorKind::Other, "Cannot write text solution with unassigned jobs."));
    }

    writer.write("Solution\n".as_bytes())?;

    solution.routes.iter().zip(1..).for_each(|(r, i)| {
        let customers = r
            .tour
            .all_activities()
            .filter(|a| a.job.is_some())
            .map(|a| a.retrieve_job().unwrap().clone())
            .map(|job| {
                match job.as_ref() {
                    Job::Single(job) => &job.dimens,
                    Job::Multi(job) => &job.dimens,
                }
                .get_id()
                .unwrap()
                .clone()
            })
            .collect::<Vec<String>>()
            .join(" ");
        writer.write(format!("Route {}: {}\n", i, customers).as_bytes()).unwrap();
    });

    Ok(())
}
