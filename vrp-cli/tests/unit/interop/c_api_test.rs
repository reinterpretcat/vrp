use super::*;
use crate::helpers::generate::SIMPLE_PROBLEM;
use vrp_pragmatic::format::FormatError;

#[test]
fn can_use_to_string() {
    let c_str = CString::new("asd").unwrap();
    assert_eq!(to_string(c_str.as_ptr() as *const c_char), "asd".to_string());
}

#[test]
fn can_use_callback() {
    extern "C" fn success1(msg: *const c_char) {
        assert_eq!(to_string(msg), "success");
    }
    extern "C" fn failure1(_: *const c_char) {
        unreachable!()
    }
    call_back(Ok("success".to_string()), success1, failure1);

    extern "C" fn success2(_: *const c_char) {
        unreachable!()
    }
    extern "C" fn failure2(msg: *const c_char) {
        // NOTE: errors are always reported as json, see `crate::interop::json`
        let err = to_string(msg);
        assert!(err.starts_with('['));
        assert!(err.contains("E9999"));
        assert!(err.contains("some cause"));
    }
    let err = FormatError::new("E9999".to_string(), "some cause".to_string(), "some action".to_string());
    call_back(Err(vec![err].into()), success2, failure2);
}

#[test]
fn can_catch_panic_with_string_literal() {
    extern "C" fn callback(msg: *const c_char) {
        let error = to_string(msg);
        assert!(error.contains("E0006"));
        assert!(error.contains("language binding panicked"));
        assert!(error.contains("invaders detected!"));
    }
    catch_panic(callback, || panic!("invaders detected!"));
    catch_panic(callback, || panic!("invaders {}!", "detected"));
}

#[test]
fn can_get_locations() {
    extern "C" fn success(locations: *const c_char) {
        let locations = to_string(locations);
        assert!(locations.starts_with('['));
        assert!(locations.ends_with(']'));
        assert!(locations.len() > 2);
    }
    extern "C" fn failure(err: *const c_char) {
        unreachable!("got {}", to_string(err))
    }

    let problem = CString::new(SIMPLE_PROBLEM).unwrap();
    get_routing_locations(problem.as_ptr() as *const c_char, success, failure)
}

#[test]
fn can_validate_simple_problem() {
    extern "C" fn success(solution: *const c_char) {
        assert_eq!(to_string(solution), "[]")
    }
    extern "C" fn failure(err: *const c_char) {
        unreachable!("{}", to_string(err))
    }

    let problem = CString::new(SIMPLE_PROBLEM).unwrap();
    validate_pragmatic(problem.as_ptr() as *const c_char, std::ptr::null(), 0, success, failure);
}

#[test]
fn can_validate_empty_problem() {
    extern "C" fn success(solution: *const c_char) {
        unreachable!("got {}", to_string(solution))
    }
    extern "C" fn failure(err: *const c_char) {
        let err = to_string(err);
        assert!(err.contains("E0000"));
        assert!(err.contains("cause"));
        assert!(err.contains("action"));
    }

    let problem = CString::new("").unwrap();
    validate_pragmatic(problem.as_ptr() as *const c_char, std::ptr::null(), 0, success, failure);
}

#[test]
fn can_solve_problem() {
    extern "C" fn success(solution: *const c_char) {
        let solution = to_string(solution);
        assert!(solution.starts_with('{'));
        assert!(solution.ends_with('}'));
        assert!(solution.len() > 2);
    }
    extern "C" fn failure(err: *const c_char) {
        unreachable!("{}", to_string(err))
    }

    let problem = CString::new(SIMPLE_PROBLEM).unwrap();
    let config = CString::new(r#"{"termination": {"maxGenerations": 1}}"#).unwrap();

    solve_pragmatic(
        problem.as_ptr() as *const c_char,
        std::ptr::null(),
        0,
        config.as_ptr() as *const c_char,
        success,
        failure,
    );
}
