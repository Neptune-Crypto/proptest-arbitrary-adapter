# proptest-arbitrary-adapter

Provides the necessary glue to reuse an implementation of [`arbitrary::Arbitrary`][arbitrary] as a
[`proptest::strategy::Strategy`][strategy].

[arbitrary]: https://docs.rs/arbitrary/1.0.0/arbitrary/trait.Arbitrary.html

[strategy]: https://docs.rs/proptest/1.0.0/proptest/strategy/trait.Strategy.html

## Usage

Assuming you use [`test-strategy`](https://crates.io/crates/test-strategy) (which you should), using a strategy for a
type that implements `arbitrary::Arbitrary` is as simple as:

```rust
#[proptest]
fn my_test(#[strategy(arb())] my_type: MyType) {
    // …
}
```

## Origin

This code is a copy of the unmaintained crate [`proptest-arbitrary-interop`][origin], with some additional improvements
from open pull requests of the original's repository.

[origin]: https://crates.io/crates/proptest-arbitrary-interop

## Caveats

It only works with types that implement `arbitrary::Arbitrary` in a particular fashion: those conforming to the
requirements of `ArbInterop`. These are roughly "types that, when randomly-generated, don't retain pointers into the
random-data buffer wrapped by the `arbitrary::Unstructured` they are generated from". Many implementations of
`arbitrary::Arbitrary` will fit the bill, but certain kinds of "zero-copy" implementations of `arbitrary::Arbitrary`
will not work. This requirement appears to be a necessary part of the semantic model of `proptest` – generated values
have to own their pointer graph, no borrows. Patches welcome if you can figure out a way to not require it.