#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt;

use arrayvec::ArrayVec;

#[derive(Debug, Clone, Default)]
pub struct ArrayBuilder<T, const N: usize> {
    inner: arrayvec::ArrayVec<T, N>,
    excess: usize,
}

impl<T, const N: usize> ArrayBuilder<T, N> {
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
    /// Pad out the array, returning an [`Err`] if there were too many calls to [`Self::push`].
    /// The builder remains unchanged in the [`Err`] case.
    ///
    /// ```
    /// # use array_builder::ArrayBuilder;
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
            return Err(Error(ErrorInner::TooMany(self.excess)));
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
    /// # use array_builder::ArrayBuilder;
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
    /// Require exactly `N` calls to [`Self::push`].
    /// The builder remains unchanged in the [`Err`] case.
    /// ```
    /// # use array_builder::ArrayBuilder;
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
            Err(Error(ErrorInner::WrongNumber {
                expected: N,
                actual: self.inner.len() + self.excess,
            }))
        }
    }
}

/// Error when building an array from [`ArrayBuilder`].
#[derive(Debug, Clone)]
pub struct Error(ErrorInner);

#[derive(Debug, Clone)]
enum ErrorInner {
    TooMany(usize),
    WrongNumber { expected: usize, actual: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            ErrorInner::TooMany(excess) => {
                f.write_fmt(format_args!("too many elements, excess: {}", excess))
            }
            ErrorInner::WrongNumber { expected, actual } => f.write_fmt(format_args!(
                "wrong number of elements, expected {}, got {}",
                expected, actual
            )),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
