// Derived from vte crate params.rs
// Original authors: vte contributors
// License: MIT OR Apache-2.0
// Modified for Hito

//! Fixed size parameters list with optional subparameters.

use core::fmt::{self, Debug, Formatter};

pub(crate) const MAX_PARAMS: usize = 32;

#[derive(Default)]
pub struct Params {
    subparams: [u8; MAX_PARAMS],
    params: [u16; MAX_PARAMS],
    current_subparams: u8,
    len: usize,
}

impl Params {
    pub fn first(&self, default: u16) -> u16 {
        self.get(0, default)
    }

    pub fn get(&self, index: usize, default: u16) -> u16 {
        if index < self.len {
            self.params[index]
        } else {
            default
        }
    }

    pub fn to_vec(&self) -> Vec<u16> {
        self.params[..self.len].to_vec()
    }
}

impl Params {
    /// Returns the number of parameters.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if there are no parameters present.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns an iterator over all parameters and subparameters.
    #[inline]
    pub fn iter(&self) -> ParamsIter<'_> {
        ParamsIter::new(self)
    }

    /// Returns `true` if there is no more space for additional parameters.
    #[allow(unused)]
    #[inline]
    pub(crate) fn is_full(&self) -> bool {
        self.len == MAX_PARAMS
    }

    /// Clear all parameters.
    #[allow(unused)]
    #[inline]
    pub(crate) fn clear(&mut self) {
        self.current_subparams = 0;
        self.len = 0;
    }

    /// Add an additional parameter.
    #[allow(unused)]
    #[inline]
    pub(crate) fn push(&mut self, item: u16) {
        self.subparams[self.len - self.current_subparams as usize] = self.current_subparams + 1;
        self.params[self.len] = item;
        self.current_subparams = 0;
        self.len += 1;
    }

    /// Add an additional subparameter to the current parameter.
    #[allow(unused)]
    #[inline]
    pub(crate) fn extend(&mut self, item: u16) {
        self.subparams[self.len - self.current_subparams as usize] = self.current_subparams + 1;
        self.params[self.len] = item;
        self.current_subparams += 1;
        self.len += 1;
    }
}

impl<'a> IntoIterator for &'a Params {
    type IntoIter = ParamsIter<'a>;
    type Item = &'a [u16];

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Immutable subparameter iterator.
pub struct ParamsIter<'a> {
    params: &'a Params,
    index: usize,
}

impl<'a> ParamsIter<'a> {
    fn new(params: &'a Params) -> Self {
        Self { params, index: 0 }
    }
}

impl<'a> Iterator for ParamsIter<'a> {
    type Item = &'a [u16];

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.params.len() {
            return None;
        }

        // Get all subparameters for the current parameter.
        let num_subparams = self.params.subparams[self.index];
        let param = &self.params.params[self.index..self.index + num_subparams as usize];

        // Jump to the next parameter.
        self.index += num_subparams as usize;

        Some(param)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.params.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl Debug for Params {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;

        for (i, param) in self.iter().enumerate() {
            if i != 0 {
                write!(f, ";")?;
            }

            for (i, subparam) in param.iter().enumerate() {
                if i != 0 {
                    write!(f, ":")?;
                }

                subparam.fmt(f)?;
            }
        }

        write!(f, "]")
    }
}
