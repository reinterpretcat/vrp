use crate::helpers::get_test_resource;
use crate::streams::input::text::solomon::parse_solomon_format;
use std::fs::File;
use std::io::BufReader;

#[test]
fn can_read_solomon_format() -> std::io::Result<()> {
    let file = get_test_resource("data/solomon/C101.25.txt").unwrap();
    let mut reader = BufReader::new(file);

    let problem = parse_solomon_format(reader);

    Ok(())
}
