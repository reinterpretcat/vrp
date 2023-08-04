use super::*;
use crate::extensions::import::import_problem;
use std::io::BufReader;

#[test]
fn can_read_csv_problem() {
    let jobs_csv = r"
ID,LAT,LNG,DEMAND,DURATION,TW_START,TW_END
job1,52.52599,13.45413,2,5,2020-07-04T08:00:00Z,2020-07-04T12:00:00Z
job2,52.5225,13.4095,1,3,,
job2,52.5165,13.3808,-1,3,,
job3,52.5316,13.3884,3,5,2020-07-04T08:00:00Z,2020-07-04T16:00:00Z
";
    let vehicles_csv = r"
ID,LAT,LNG,CAPACITY,TW_START,TW_END,AMOUNT,PROFILE
vehicle1,52.4664,13.4023,40,2020-07-04T08:00:00Z,2020-07-04T20:00:00Z,10,car
vehicle2,52.4959,13.3539,50,2020-07-04T08:00:00Z,2020-07-04T20:00:00Z,20,truck
";

    let result = read_csv_problem(BufReader::new(jobs_csv.as_bytes()), BufReader::new(vehicles_csv.as_bytes()))
        .expect("cannot read csv");

    assert_eq!(result.plan.jobs.len(), 3);
    assert_eq!(result.fleet.vehicles.len(), 2);
}

#[test]
fn can_propagate_format_error() {
    let invalid_jobs = r"
ID,LAT,LNG,DEMAND,DURATION,TW_START,TW_END
job2,52.5225,13.4095,1,3,,
job2,52.5165,13.3808,3,,
";

    let result = read_csv_problem(BufReader::new(invalid_jobs.as_bytes()), BufReader::new("".as_bytes()))
        .expect_err("Should return error!");

    assert_eq!(result.code, "E0000");
    assert_eq!(result.cause, "cannot read jobs");
    assert_eq!(result.action, "check jobs definition");
    assert!(result.details.is_some());

    let result =
        import_problem("csv", Some(vec![BufReader::new(invalid_jobs.as_bytes()), BufReader::new("".as_bytes())]))
            .expect_err("Should return error!")
            .to_string();

    assert_eq!(result, "cannot read csv: E0000, cause: 'cannot read jobs', action: 'check jobs definition'.");
}

parameterized_test! {can_handle_invalid_input_amount, input_size, {
        can_handle_invalid_input_amount_impl(input_size);
}}

can_handle_invalid_input_amount! {
        case01: None,
        case02: Some(0),
        case03: Some(1),
        case04: Some(3),
}

fn can_handle_invalid_input_amount_impl(input_size: Option<usize>) {
    let jobs_csv = r"
ID,LAT,LNG,DEMAND,DURATION,TW_START,TW_END
job1,52.5225,13.4095,1,3,,
";

    let result =
        import_problem("csv", input_size.map(|size| (0..size).map(|_| BufReader::new(jobs_csv.as_bytes())).collect()))
            .expect_err("Should return error!")
            .to_string();

    assert_eq!(result, "csv format expects two files with jobs and vehicles as an input");
}
