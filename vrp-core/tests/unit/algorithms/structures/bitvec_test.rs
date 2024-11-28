use super::BitVec;

#[test]
fn can_create_new() {
    let bitvec = BitVec::new(10);
    assert_eq!(bitvec.len(), 10);
    assert!(bitvec.blocks.iter().all(|&block| block == 0));
}

#[test]
fn can_use_set_and_get() {
    let mut bitvec = BitVec::new(10);
    bitvec.set(3, true);
    assert_eq!(bitvec.get(3), Some(true));
    bitvec.set(3, false);
    assert_eq!(bitvec.get(3), Some(false));
}

#[test]
#[should_panic]
fn can_panic_when_set_out_of_bounds() {
    let mut bitvec = BitVec::new(10);
    bitvec.set(10, true);
}

#[test]
fn can_use_union() {
    let mut bitvec1 = BitVec::new(10);
    let mut bitvec2 = BitVec::new(10);
    bitvec1.set(3, true);
    bitvec2.set(4, true);
    bitvec1.union(&bitvec2);
    assert_eq!(bitvec1.get(3), Some(true));
    assert_eq!(bitvec1.get(4), Some(true));
}

#[test]
fn can_use_len_and_is_empty() {
    let bitvec = BitVec::new(0);
    assert_eq!(bitvec.len(), 0);
    assert!(bitvec.is_empty());
}

#[test]
fn can_index() {
    let mut bitvec = BitVec::new(10);
    bitvec.set(3, true);
    assert!(bitvec[3]);
    assert!(!bitvec[4]);
}

#[test]
fn can_use_display() {
    let mut bitvec = BitVec::new(5);
    bitvec.set(0, true);
    bitvec.set(2, true);
    assert_eq!(format!("{}", bitvec), "[10100]");
}
