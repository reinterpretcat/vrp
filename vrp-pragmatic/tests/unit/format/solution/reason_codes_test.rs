use super::*;

/// Every constraint code this crate defines must have a name and a sentence a dispatcher can act
/// on. The mapping ends in a catch-all that answers `("NO_REASON_FOUND", "unknown")`, so a code
/// added without a matching arm does not fail anywhere — it silently reaches the operator as
/// "unknown", which is what `VEHICLE_GROUP_CONSTRAINT_CODE` did from the day it was introduced.
///
/// The codes are dense and start at 1, so walking the range is the whole population.
#[test]
fn every_defined_code_is_named() {
    let highest = VEHICLE_GROUP_CONSTRAINT_CODE.0;

    let unnamed = (1..=highest)
        .map(|code| (code, map_code_reason(ViolationCode(code))))
        .filter(|(_, (name, _))| *name == "NO_REASON_FOUND")
        .map(|(code, _)| code)
        .collect::<Vec<_>>();

    assert!(unnamed.is_empty(), "violation codes reaching the operator as 'unknown': {unnamed:?}");
}

/// The reverse direction reads a solution back in as a warm start, so a name the writer emits and
/// the reader cannot parse would silently lose the reason on the round trip.
#[test]
fn every_named_code_round_trips() {
    let highest = VEHICLE_GROUP_CONSTRAINT_CODE.0;

    for code in 1..=highest {
        let (name, _) = map_code_reason(ViolationCode(code));
        assert_eq!(map_reason_code(name), ViolationCode(code), "'{name}' does not read back as code {code}");
    }
}
