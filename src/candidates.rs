// MIT License
//
// Copyright (c) 2026 worksoup <https://github.com/worksoup/>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::ffi::CStr;

use crate::{Error, UCharsetDetector};

pub struct Candidates {
    pub(crate) detector: UCharsetDetector,
    pub(crate) n_candidates: usize,
}

impl Candidates {
    pub fn detect(data: impl AsRef<[u8]>) -> Result<Candidates, Error> {
        UCharsetDetector::detect_data(data)
    }

    pub fn detector(&self) -> &UCharsetDetector {
        &self.detector
    }

    pub fn reset(mut self) -> UCharsetDetector {
        self.detector.reset();
        self.detector
    }

    pub fn len(&self) -> usize {
        self.n_candidates
    }

    pub fn is_empty(&self) -> bool {
        self.n_candidates == 0
    }

    pub fn get(&self, index: usize) -> Option<Candidate<'_>> {
        if index < self.n_candidates {
            Some(Candidate {
                parent: self,
                index,
            })
        } else {
            None
        }
    }

    pub fn best(&self) -> Option<Candidate<'_>> {
        self.get(0)
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self)
    }
}

pub struct Candidate<'a> {
    parent: &'a Candidates,
    index: usize,
}

impl<'a> Candidate<'a> {
    #[cfg(feature = "encoding")]
    pub fn encoding(&self) -> Result<&'static encoding_rs::Encoding, Error> {
        crate::encoding::to_standard(self.encoding_name()?).ok_or(Error::NonStandardCharset)
    }

    pub fn encoding_name(&self) -> Result<&'a str, Error> {
        let ptr = unsafe { sys::uchardet_get_encoding(self.parent.detector.ptr, self.index) };
        debug_assert!(!ptr.is_null());
        unsafe { CStr::from_ptr(ptr) }
            .to_str()
            .map_err(|_| Error::InvalidCharset)
            .and_then(|s| {
                if s.is_empty() {
                    Err(Error::UnrecognizableCharset)
                } else {
                    Ok(s)
                }
            })
    }

    pub fn confidence(&self) -> f32 {
        unsafe { sys::uchardet_get_confidence(self.parent.detector.ptr, self.index) }
    }

    pub fn language(&self) -> Result<Option<&'a str>, Error> {
        let ptr = unsafe { sys::uchardet_get_language(self.parent.detector.ptr, self.index) };
        if ptr.is_null() {
            return Ok(None);
        }
        let s = unsafe { CStr::from_ptr(ptr) }.to_str()?;
        Ok(if s.is_empty() { None } else { Some(s) })
    }
}

pub struct Iter<'a> {
    parent: &'a Candidates,
    start: usize,
    end: usize,
}

impl<'a> Iter<'a> {
    fn new(parent: &'a Candidates) -> Self {
        Iter {
            parent,
            start: 0,
            end: parent.n_candidates,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Candidate<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            let item = Candidate {
                parent: self.parent,
                index: self.start,
            };
            self.start += 1;
            Some(item)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.end - self.start;
        (len, Some(len))
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            self.end -= 1;
            Some(Candidate {
                parent: self.parent,
                index: self.end,
            })
        } else {
            None
        }
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {
    fn len(&self) -> usize {
        self.end - self.start
    }
}

impl<'a> IntoIterator for &'a Candidates {
    type Item = Candidate<'a>;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
