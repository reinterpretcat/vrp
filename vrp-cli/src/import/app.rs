use super::*;

pub fn get_import_app<'a, 'b>() -> App<'a, 'b> {
    App::new("import").about("Provides the way to import problem from various formats")
}
