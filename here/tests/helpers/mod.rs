use std::fs::File;

pub fn get_test_resource(resource_path: &str) -> std::io::Result<File> {
    let mut path = std::env::current_dir()?;
    path.push("tests");
    path.push(resource_path);

    File::open(path)
}
