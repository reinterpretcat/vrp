use super::*;

#[test]
fn can_read_jobs() {
    let jobs_csv = r"
ID,LAT,LNG,DEMAND,DURATION,TW_START,TW_END
job1,52.52599,13.45413,2,5,2020-07-04T08:00:00Z,2020-07-04T12:00:00Z
job2,52.5225,13.4095,1,3,,
job2,52.5165,13.3808,-1,3,,
job3,52.5316,13.3884,3,5,2020-07-04T08:00:00Z,2020-07-04T16:00:00Z
";

    let jobs = read_jobs(BufReader::new(jobs_csv.as_bytes())).unwrap();

    assert_eq!(jobs.len(), 3);
}

#[test]
fn can_read_vehicles() {
    let vehicles_csv = r"
ID,LAT,LNG,CAPACITY,TW_START,TW_END,AMOUNT,PROFILE
vehicle1,52.4664,13.4023,40,2020-07-04T08:00:00Z,2020-07-04T20:00:00Z,10,car
vehicle2,52.4959,13.3539,50,2020-07-04T08:00:00Z,2020-07-04T20:00:00Z,20,truck
";

    let vehicles = read_vehicles(BufReader::new(vehicles_csv.as_bytes())).unwrap();

    assert_eq!(vehicles.len(), 2);
}

#[test]
fn can_propagate_format_error() {
    let invalid_jobs = r"
ID,LAT,LNG,DEMAND,DURATION,TW_START,TW_END
job2,52.5225,13.4095,1,3,,
job2,52.5165,13.3808,3,,
";

    let result = read_csv_problem(BufReader::new(invalid_jobs.as_bytes()), BufReader::new("".as_bytes()))
        .err()
        .expect("Should return error!");

    assert_eq!(result.code, "E0000");
    assert_eq!(result.cause, "cannot read jobs");
    assert_eq!(result.action, "check jobs definition");
    assert!(result.details.is_some())
}
