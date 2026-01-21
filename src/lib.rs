//! Provides the necessary glue to reuse an implementation of
//! [`arbitrary::Arbitrary`] as a [`proptest::strategy::Strategy`].
//!
//! # Usage
//!
//! Assuming you use [`test-strategy`](https://crates.io/crates/test-strategy)
//! (which you should), using a strategy for a type that implements
//! [`arbitrary::Arbitrary`] is as simple as:
//!
//! ```rust
//! # use test_strategy::proptest;
//! # #[derive(Debug, Clone, arbitrary::Arbitrary)]
//! # struct MyType(u8);
//! #
//! #[proptest]
//! fn my_test(#[strategy(arb())] my_type: MyType) {
//!     // …
//! }
//! ```
//!
//! # Origin
//!
//! This code is a copy of the unmaintained crate
//! [`proptest-arbitrary-interop`][origin], with some additional improvements
//! from open pull requests of the original's repository.
//!
//! [origin]: https://crates.io/crates/proptest-arbitrary-interop
//!
//! # Caveats
//!
//! It only works with types that implement [`arbitrary::Arbitrary`] in a
//! particular fashion: those conforming to the requirements of [`ArbInterop`].
//! These are roughly "types that, when randomly-generated, don't retain
//! pointers into the random-data buffer wrapped by the
//! [`arbitrary::Unstructured`] they are generated from". Many implementations
//! of [`arbitrary::Arbitrary`] will fit the bill, but certain kinds of
//! "zero-copy" implementations of [`arbitrary::Arbitrary`] will not work. This
//! requirement appears to be a necessary part of the semantic model of
//! [`proptest`] – generated values have to own their pointer graph, no
//! borrows. Patches welcome if you can figure out a way to not require it.

use core::fmt::Debug;
use std::marker::PhantomData;

use proptest::prelude::RngCore;
use proptest::test_runner::TestRunner;

/// The subset of possible [`arbitrary::Arbitrary`] implementations that this
/// crate works with. The main concern here is the `for<'a> Arbitrary<'a>`
/// business, which (in practice) decouples the generated `Arbitrary` value from
/// the lifetime of the random buffer it's fed; I can't actually explain how,
/// because Rust's type system is way over my head.
pub trait ArbInterop: for<'a> arbitrary::Arbitrary<'a> + 'static + Debug + Clone {}
impl<A> ArbInterop for A where A: for<'a> arbitrary::Arbitrary<'a> + 'static + Debug + Clone {}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ArbStrategy<A: ArbInterop> {
    size: usize,
    _ph: PhantomData<A>,
}

#[derive(Debug)]
pub struct ArbValueTree<A: Debug> {
    bytes: Vec<u8>,
    curr: A,
    prev: Option<A>,
    next: usize,
}

impl<A: ArbInterop> proptest::strategy::ValueTree for ArbValueTree<A> {
    type Value = A;

    fn current(&self) -> Self::Value {
        self.curr.clone()
    }

    fn simplify(&mut self) -> bool {
        if self.next == 0 {
            return false;
        }
        self.next -= 1;
        let Ok(simpler) = Self::gen_one_with_size(&self.bytes, self.next) else {
            return false;
        };

        // Throw away the previous value and set the current value as prev.
        // Advance the iterator and set the current value to the next one.
        self.prev = Some(core::mem::replace(&mut self.curr, simpler));

        true
    }

    fn complicate(&mut self) -> bool {
        // We can only complicate if we previously simplified. Complicating
        // twice in a row without interleaved simplification is guaranteed to
        // always yield false for the second call.
        let Some(prev) = self.prev.take() else {
            return false;
        };

        // Throw away the current value!
        self.curr = prev;

        true
    }
}

impl<A: ArbInterop> ArbStrategy<A> {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            _ph: PhantomData,
        }
    }
}

impl<A: ArbInterop> ArbValueTree<A> {
    fn gen_one_with_size(bytes: &[u8], size: usize) -> Result<A, arbitrary::Error> {
        A::arbitrary(&mut arbitrary::Unstructured::new(&bytes[0..size]))
    }

    pub fn new(bytes: Vec<u8>) -> Result<Self, arbitrary::Error> {
        let next = bytes.len();
        let curr = Self::gen_one_with_size(&bytes, next)?;

        Ok(Self {
            bytes,
            prev: None,
            curr,
            next,
        })
    }
}

impl<A: ArbInterop> proptest::strategy::Strategy for ArbStrategy<A> {
    type Tree = ArbValueTree<A>;
    type Value = A;

    fn new_tree(&self, run: &mut TestRunner) -> proptest::strategy::NewTree<Self> {
        loop {
            let mut bytes = vec![0; self.size];
            run.rng().fill_bytes(&mut bytes);
            match ArbValueTree::new(bytes) {
                Ok(v) => return Ok(v),

                // If the Arbitrary impl cannot construct a value from the given
                // bytes, try again.
                Err(e @ arbitrary::Error::IncorrectFormat) => run.reject_local(format!("{e}"))?,
                Err(e) => return Err(format!("{e}").into()),
            }
        }
    }
}

/// Constructs a [`proptest::strategy::Strategy`] for a given
/// [`arbitrary::Arbitrary`] type, generating `size` bytes of random data as
/// input to the [`arbitrary::Arbitrary`] type.
pub fn arb_sized<A: ArbInterop>(size: usize) -> ArbStrategy<A> {
    ArbStrategy::new(size)
}

/// Constructs a [`proptest::strategy::Strategy`] for a given
/// [`arbitrary::Arbitrary`] type.
///
/// Calls [`arb_sized`] with a best-effort guess for the size, generating `size`
/// bytes of random data as input to the [`arbitrary::Arbitrary`] type.
///
/// In particular, if `A`'s [`size_hint`](arbitrary::Arbitrary::size_hint) is
/// useful, the hint is used; otherwise, a default size of 256 is used.
pub fn arb<A: ArbInterop>() -> ArbStrategy<A> {
    let (low, opt_high) = A::size_hint(0);
    let Some(high) = opt_high else {
        let size_hint = (2 * low).max(256);
        return arb_sized(size_hint);
    };

    arb_sized(high)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, feature(coverage_attribute))]
mod tests {
    use arbitrary::Arbitrary;
    use proptest::prelude::*;
    use test_strategy::proptest;

    use super::*;

    #[derive(Debug, Clone, Arbitrary)]
    struct Test(u8);

    #[proptest(cases = 1)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    fn type_can_be_generated(#[strategy(arb())] test: Test) {
        let Test(_t) = test;
    }

    // As far as I know, `wasm_bindgen_test` does not support  the
    // `#[should_panic]` attribute:
    // https://github.com/wasm-bindgen/wasm-bindgen/issues/2286
    #[should_panic]
    #[proptest(cases = 1)]
    fn type_can_shrink(#[strategy(arb())] _test: Test) {
        Err(TestCaseError::Fail("always".into()))?;
    }
}
