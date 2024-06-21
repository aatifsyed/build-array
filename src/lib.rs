#![cfg_attr(not(feature = "std"), no_std)]

use core::{
    cmp, fmt,
    mem::{self, MaybeUninit},
};

pub struct ArrayBuilder<T, const N: usize> {
    inner: [MaybeUninit<T>; N],
    initialised: usize,
}

impl<T, const N: usize> fmt::Debug for ArrayBuilder<T, N>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArrayBuilder")
            .field("initialised", &self.as_init_slice())
            .field("current_len", &self.as_init_slice().len())
            .field("target_len", &N)
            .finish()
    }
}

impl<T, const N: usize> Default for ArrayBuilder<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Drop for ArrayBuilder<T, N> {
    fn drop(&mut self) {
        for i in 0..cmp::min(self.initialised, N) {
            unsafe { self.inner[i].assume_init_drop() }
        }
    }
}

impl<T, const N: usize> ArrayBuilder<T, N> {
    fn as_init_slice(&self) -> &[T] {
        unsafe { mem::transmute(&self.inner[..self.initialised]) }
    }
    pub const fn new() -> Self {
        Self {
            inner: [const { MaybeUninit::uninit() }; N],
            initialised: 0,
        }
    }
    pub fn push(&mut self, item: T) {
        if let Some(dst) = self.inner.get_mut(self.initialised) {
            dst.write(item);
        }
        self.initialised += 1
    }
    pub fn with(mut self, item: T) -> Self {
        self.push(item);
        self
    }
    fn pad_with(&mut self, mut f: impl FnMut() -> T) {
        for _ in self.initialised..N {
            self.push(f())
        }
    }
    /// Pad out the array, returning an [`Err`] if there were too many calls to [`Self::push`].
    ///
    /// ```
    /// # use array_builder::ArrayBuilder;
    /// let arr = ArrayBuilder::<_, 3>::new().with("first").build_pad("padding").unwrap();
    /// assert_eq!(arr, ["first", "padding", "padding"]);
    ///
    /// ArrayBuilder::<_, 1>::new().with("first").with("too many now!").build_pad("").unwrap_err();
    /// ```
    pub fn build_pad(mut self, item: T) -> Result<[T; N], Error<T, N, TooManyElements>>
    where
        T: Clone,
    {
        self.pad_with(|| item.clone());
        match self.initialised == N {
            true => {
                self.initialised = 0;
                Ok(unsafe { mem::transmute_copy(&self.inner) })
            }
            false => Err(Error {
                reason: TooManyElements {
                    expected: N,
                    actual: self.initialised,
                },
                builder: self,
            }),
        }
    }
    /// Pad out the array, ignoring if there were too many calls to [`Self::push`].
    /// ```
    /// # use array_builder::ArrayBuilder;
    /// let arr = ArrayBuilder::<_, 3>::new().with("first").build_pad_truncate("padding");
    /// assert_eq!(arr, ["first", "padding", "padding"]);
    ///
    /// let arr =
    ///     ArrayBuilder::<_, 1>::new().with("first").with("too many now!").build_pad_truncate("");
    /// assert_eq!(arr, ["first"]);
    /// ```
    pub fn build_pad_truncate(mut self, item: T) -> [T; N]
    where
        T: Clone,
    {
        self.pad_with(|| item.clone());
        self.initialised = 0;
        unsafe { mem::transmute_copy(&self.inner) }
    }
    /// Require `N` calls to [`Self::push`].
    /// ```
    /// # use array_builder::ArrayBuilder;
    ///
    /// ArrayBuilder::<_, 2>::new().with("too few").build_exact().unwrap_err();
    /// ArrayBuilder::<_, 2>::new().with("way").with("too").with("many").build_exact().unwrap_err();
    /// ArrayBuilder::<_, 2>::new().with("just").with("right").build_exact().unwrap();
    /// ```
    pub fn build_exact(mut self) -> Result<[T; N], Error<T, N>> {
        match self.initialised == N {
            true => {
                self.initialised = 0;
                Ok(unsafe { mem::transmute_copy(&self.inner) })
            }
            false => Err(Error {
                reason: WrongNumberOfElements {
                    expected: N,
                    actual: self.initialised,
                },
                builder: self,
            }),
        }
    }
}

/// Error from [`ArrayBuilder::build_pad`]
#[derive(Debug, Clone)]
pub struct TooManyElements {
    expected: usize,
    actual: usize,
}

impl fmt::Display for TooManyElements {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "too many elements, expected {}, got {}",
            self.expected, self.actual
        ))
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TooManyElements {}

/// Error from [`ArrayBuilder::build_exact`]
#[derive(Debug, Clone)]
pub struct WrongNumberOfElements {
    expected: usize,
    actual: usize,
}

impl fmt::Display for WrongNumberOfElements {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "wrong number of elements, expected {}, got {}",
            self.expected, self.actual
        ))
    }
}

#[cfg(feature = "std")]
impl std::error::Error for WrongNumberOfElements {}

/// Build error, coupled with the [`ArrayBuilder`] that failed.
#[derive(Debug)]
#[non_exhaustive]
pub struct Error<T, const N: usize, R = WrongNumberOfElements> {
    /// The rejected builder
    pub builder: ArrayBuilder<T, N>,
    /// The reason building failed
    pub reason: R,
}

impl<T, const N: usize, R> fmt::Display for Error<T, N, R>
where
    R: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        R::fmt(&self.reason, f)
    }
}

#[cfg(feature = "std")]
impl<T, const N: usize, R> std::error::Error for Error<T, N, R>
where
    R: std::error::Error,
    T: fmt::Debug,
{
}
