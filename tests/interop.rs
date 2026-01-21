// Part 1: suppose you implement Arbitrary for one of your types
// because you want to fuzz it.

#[derive(Copy, Clone, Debug, arbitrary::Arbitrary)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

// Part 2: suppose you later decide that in addition to fuzzing
// you want to use that Arbitrary impl, but with proptest.

use proptest::prelude::*;
use proptest_arbitrary_adapter::arb;
use test_strategy::proptest;

#[proptest]
#[should_panic]
fn always_red(#[strategy(arb())] color: Rgb) {
    prop_assert!(color.g == 0 || color.r > color.g);
}
