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

use std::ffi::{CString, c_char};

use crate::{Candidates, Error};

pub struct CharsetDetector {
    pub(crate) ptr: sys::uchardet_t,
    pub(crate) external_error_occurred: bool,
}

impl Default for CharsetDetector {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl CharsetDetector {
    pub fn new() -> CharsetDetector {
        let ptr = unsafe { sys::uchardet_new() };
        debug_assert!(!ptr.is_null());
        CharsetDetector {
            ptr,
            external_error_occurred: false,
        }
    }

    pub fn feed_data(&mut self, data: impl AsRef<[u8]>) -> Result<(), Error> {
        let data = data.as_ref();
        let ret = unsafe {
            sys::uchardet_handle_data(self.ptr, data.as_ptr() as *const c_char, data.len())
        };
        if ret == 0 {
            Ok(())
        } else {
            self.external_error_occurred = true;
            Err(unsafe { Error::from_ret(ret) })
        }
    }

    pub fn reset(&mut self) {
        self.external_error_occurred = false;
        unsafe { sys::uchardet_reset(self.ptr) };
    }

    pub fn detect(self) -> Candidates {
        unsafe {
            sys::uchardet_data_end(self.ptr);
        };
        let n_candidates = unsafe { sys::uchardet_get_n_candidates(self.ptr) };
        Candidates {
            detector: self,
            n_candidates,
        }
    }

    pub fn detect_data(data: impl AsRef<[u8]>) -> Result<Candidates, Error> {
        let mut detector = CharsetDetector::new();
        detector.feed_data(data.as_ref())?;
        Ok(detector.detect())
    }

    pub fn weigh_language(&mut self, language: &str, weight: f32) -> Result<(), Error> {
        let c_lang = CString::new(language)?;
        unsafe {
            sys::uchardet_weigh_language(self.ptr, c_lang.as_ptr(), weight);
        }
        Ok(())
    }

    pub fn set_default_weight(&mut self, weight: f32) {
        unsafe { sys::uchardet_set_default_weight(self.ptr, weight) }
    }

    pub fn external_error_occurred(&self) -> bool {
        self.external_error_occurred
    }
}

impl Drop for CharsetDetector {
    fn drop(&mut self) {
        unsafe { sys::uchardet_delete(self.ptr) };
    }
}
