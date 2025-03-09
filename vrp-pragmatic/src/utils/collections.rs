/// Combines error results.
pub fn combine_error_results<T: Clone>(results: &[Result<(), T>]) -> Result<(), Vec<T>> {
    let errors = results.iter().cloned().flat_map(|result| result.err().into_iter()).collect::<Vec<T>>();

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
