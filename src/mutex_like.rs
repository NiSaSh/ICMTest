use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
};

/// Mutex-like wrapper, but it actually does not perform any locking (so there are no performance
/// overheads). Use this wrapper when:
///   1. Sync and the interior mutability is needed, and
///   2. it is (manually) guaranteed that data races will not occur.
#[derive(Debug)]
pub struct MutexLike<T: ?Sized> {
    data: UnsafeCell<T>,
}

/// Smart pointer like wrapper that is returned when `MutexLike` is "locked".
#[derive(Debug)]
pub struct MutexGuardLike<'a, T: ?Sized + 'a> {
    mutex: &'a MutexLike<T>,
}

unsafe impl<T: ?Sized + Send> Send for MutexLike<T> {}
unsafe impl<T: ?Sized + Send> Sync for MutexLike<T> {}
unsafe impl<'a, T: ?Sized + Sync + 'a> Sync for MutexGuardLike<'a, T> {}

impl<T> MutexLike<T> {
    #[inline]
    pub fn new(val: T) -> Self {
        Self {
            data: UnsafeCell::new(val),
        }
    }
}

impl<T: ?Sized> MutexLike<T> {
    #[inline]
    pub fn lock(&self) -> MutexGuardLike<T> {
        MutexGuardLike { mutex: self }
    }
}

impl<T: ?Sized + Default> Default for MutexLike<T> {
    #[inline]
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<'a, T: ?Sized + 'a> Deref for MutexGuardLike<'a, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T: ?Sized + 'a> DerefMut for MutexGuardLike<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}