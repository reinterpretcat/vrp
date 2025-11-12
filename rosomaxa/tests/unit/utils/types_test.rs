use super::*;

mod bit_array {
    use super::*;

    #[test]
    fn can_use_fixed_bit_array() {
        let mut bit_array = FixedBitArray::<4>::default();

        assert_eq!(format!("{bit_array:b}"), "00000000 00000000 00000000 00000000");

        bit_array.set(1, true);
        assert_eq!(format!("{bit_array:b}"), "01000000 00000000 00000000 00000000");

        bit_array.set(31, true);
        assert_eq!(format!("{bit_array:b}"), "01000000 00000000 00000000 00000001");

        assert!(bit_array.get(1));
        assert!(bit_array.get(31));

        assert!(!bit_array.get(2));

        assert!(!bit_array.set(100, true));
        assert!(!bit_array.get(32));

        assert!(!bit_array.set(100, true));
        assert!(!bit_array.get(100));
    }

    #[test]
    fn can_set_unset_bits() {
        let mut bit_array = FixedBitArray::<4>::default();

        bit_array.set(9, true);
        assert!(bit_array.get(9));
        assert_eq!(format!("{bit_array:b}"), "00000000 01000000 00000000 00000000");
        bit_array.set(9, false);
        assert!(!bit_array.get(9));
        assert_eq!(format!("{bit_array:b}"), "00000000 00000000 00000000 00000000");
    }

    #[test]
    fn can_use_replace() {
        let mut bit_array = FixedBitArray::<4>::default();

        assert!(!bit_array.replace(1, true));
        assert_eq!(format!("{bit_array:b}"), "01000000 00000000 00000000 00000000");

        assert!(!bit_array.replace(31, true));
        assert_eq!(format!("{bit_array:b}"), "01000000 00000000 00000000 00000001");

        assert!(bit_array.replace(31, false));
        assert_eq!(format!("{bit_array:b}"), "01000000 00000000 00000000 00000000");

        assert!(!bit_array.replace(32, true));
        assert_eq!(format!("{bit_array:b}"), "01000000 00000000 00000000 00000000");
        assert!(!bit_array.replace(32, false));
    }
}
