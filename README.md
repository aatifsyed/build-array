<!-- cargo-rdme start -->

Build an array dynamically without heap allocations, deferring errors to a
single `build` callsite.

```rust
let arr: [u8; 3] = ArrayBuilder::new()
    .push(1)
    .push(2)
    .push(3)
    .build_exact()
    .unwrap();

assert_eq!(arr, [1, 2, 3]);
```

You can choose how to handle the wrong number of [`push`](ArrayBuilder::push)
calls:
- [build_exact](ArrayBuilder::build_exact).
- [build_pad](ArrayBuilder::build_pad).
- [build_pad_truncate](ArrayBuilder::build_pad_truncate).

# Comparison with other libraries
- [arrayvec] requires you to handle over-provision at each call to [`try_push`](arrayvec::ArrayVec::try_push).
- [array_builder](https://docs.rs/array_builder/latest/array_builder/) will
  [`panic!`] on over-provision.

<!-- cargo-rdme end -->
