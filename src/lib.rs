//! Build an array dynamically without heap allocations, deferring errors to a
//! single `build` callsite.
//!
//! ```
//! # use build_array::ArrayBuilder;
//! let arr: [u8; 3] = ArrayBuilder::new()
//!     .push(1)
//!     .push(2)
//!     .push(3)
//!     .build_exact()
//!     .unwrap();
//!
//! assert_eq!(arr, [1, 2, 3]);
//! ```
//!
//! You can choose how to handle the wrong number of [`push`](ArrayBuilder::push)
//! calls:
//! - [`build_exact`](ArrayBuilder::build_exact).
//! - [`build_pad`](ArrayBuilder::build_pad).
//! - [`build_truncate`](ArrayBuilder::build_truncate).
//! - [`build_pad_truncate`](ArrayBuilder::build_pad_truncate).
//!
//! # Comparison with other libraries
//! - [`arrayvec`] requires you to handle over-provision at each call to [`try_push`](arrayvec::ArrayVec::try_push).
//! - [`array_builder`](https://docs.rs/array_builder/latest/array_builder/) will
//!   [`panic!`] on over-provision.

#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt;

use arrayvec::ArrayVec;

/// Shorthand for [`ArrayBuilder::new`].
///
/// ```
/// # let arr: [&str; 0] =
/// build_array::new()
///     .push("hello")
///     .build_pad_truncate("pad");
/// ```
pub const fn new<T, const N: usize>() -> ArrayBuilder<T, N> {
    ArrayBuilder::new()
}

/// Build an array dynamically without heap allocations.
///
/// See [module documentation](mod@self) for more.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayBuilder<T, const N: usize> {
    inner: arrayvec::ArrayVec<T, N>,
    excess: usize,
}

impl<T, const N: usize> ArrayBuilder<T, N> {
    /// Create a new, empty builder.
    pub const fn new() -> Self {
        Self {
            inner: ArrayVec::new_const(),
            excess: 0,
        }
    }
    /// Insert an item into the builder.
    ///
    /// If the builder is already full, the item is immediately dropped.
    pub fn push(&mut self, item: T) -> &mut Self {
        if self.inner.try_push(item).is_err() {
            self.excess += 1
        };
        self
    }
    fn pad_with(&mut self, mut f: impl FnMut() -> T) {
        for _ in 0..self.inner.remaining_capacity() {
            self.inner.push(f())
        }
    }
    fn error(&self) -> Error {
        Error {
            expected: N,
            actual: self.inner.len() + self.excess,
        }
    }
    /// Pad out the array, returning an [`Err`] if there were too many calls to [`Self::push`].
    /// The builder remains unchanged in the [`Err`] case.
    ///
    /// ```
    /// # use build_array::ArrayBuilder;
    /// let arr = ArrayBuilder::<_, 3>::new().push("first").build_pad("padding").unwrap();
    /// assert_eq!(arr, ["first", "padding", "padding"]);
    ///
    /// ArrayBuilder::<_, 1>::new().push("first").push("too many now!").build_pad("").unwrap_err();
    /// ```
    pub fn build_pad(&mut self, item: T) -> Result<[T; N], Error>
    where
        T: Clone,
    {
        if self.excess > 0 {
            return Err(self.error());
        }
        self.pad_with(|| item.clone());
        match self.inner.take().into_inner() {
            Ok(it) => Ok(it),
            Err(_) => unreachable!("we've just padded"),
        }
    }
    /// Pad out the array, ignoring if there were too many calls to [`Self::push`].
    /// The builder is restored to an empty state.
    ///
    /// ```
    /// # use build_array::ArrayBuilder;
    /// let arr = ArrayBuilder::<_, 3>::new().push("first").build_pad_truncate("padding");
    /// assert_eq!(arr, ["first", "padding", "padding"]);
    ///
    /// let arr =
    ///     ArrayBuilder::<_, 1>::new().push("first").push("too many now!").build_pad_truncate("");
    /// assert_eq!(arr, ["first"]);
    /// ```
    pub fn build_pad_truncate(&mut self, item: T) -> [T; N]
    where
        T: Clone,
    {
        self.pad_with(|| item.clone());
        self.excess = 0;
        match self.inner.take().into_inner() {
            Ok(it) => it,
            Err(_) => unreachable!("we've just padded"),
        }
    }
    /// Build the array, ignoring if there were too many calls to [`Self::push`].
    /// The builder is restored to an empty state, and remains unchanged in the
    /// [`Err`] case.
    ///
    /// ```
    /// # use build_array::ArrayBuilder;
    /// let arr = ArrayBuilder::<_, 1>::new().push("first").push("ignored").build_truncate().unwrap();
    /// assert_eq!(arr, ["first"]);
    ///
    /// ArrayBuilder::<&str, 1>::new().build_truncate().unwrap_err();
    /// ```
    pub fn build_truncate(&mut self) -> Result<[T; N], Error> {
        match self.inner.remaining_capacity() == 0 {
            true => match self.inner.take().into_inner() {
                Ok(it) => Ok(it),
                Err(_) => unreachable!("we've just checked the capacity"),
            },
            false => Err(self.error()),
        }
    }

    /// Require exactly `N` calls to [`Self::push`].
    /// The builder remains unchanged in the [`Err`] case.
    /// ```
    /// # use build_array::ArrayBuilder;
    ///
    /// ArrayBuilder::<_, 2>::new().push("too few").build_exact().unwrap_err();
    /// ArrayBuilder::<_, 2>::new().push("way").push("too").push("many").build_exact().unwrap_err();
    /// ArrayBuilder::<_, 2>::new().push("just").push("right").build_exact().unwrap();
    /// ```
    pub fn build_exact(&mut self) -> Result<[T; N], Error> {
        if self.inner.remaining_capacity() == 0 && self.excess == 0 {
            match self.inner.take().into_inner() {
                Ok(it) => Ok(it),
                Err(_) => unreachable!("remaining capacity is zero"),
            }
        } else {
            Err(self.error())
        }
    }
    /// Return the current collection of items in the array.
    ///
    /// Does not include excess items.
    pub fn as_slice(&self) -> &[T] {
        self.inner.as_slice()
    }
    /// Return the current collection of items in the array.
    ///
    /// Does not include excess items.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.inner.as_mut_slice()
    }
}

impl<T, const N: usize> Extend<T> for ArrayBuilder<T, N> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for it in iter {
            self.push(it);
        }
    }
}
impl<T, const N: usize> FromIterator<T> for ArrayBuilder<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut this = Self::new();
        this.extend(iter);
        this
    }
}

/// Error when building an array from [`ArrayBuilder`].
#[derive(Debug, Clone)]
pub struct Error {
    expected: usize,
    actual: usize,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { expected, actual } = self;
        let snip = match actual < expected {
            true => "few",
            false => "many",
        };
        f.write_fmt(format_args!(
            "too {} elements for array, needed {} but got {}",
            snip, expected, actual
        ))
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
