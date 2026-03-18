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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the detector could not determine the character encoding")]
    UnrecognizableCharset,
    #[error("the detector products a invalid charset name.")]
    InvalidCharset,
    #[error("a non-standard charset name that encoding_rs doesn't support: .")]
    NonStandardCharset,

    #[error("out of memory, underlayer error code is {0}")]
    OutOfMemory(i32),

    #[error("invalid language string: {0}")]
    InvalidLanguage(#[from] std::ffi::NulError),
    #[error("invalid language string: {0}")]
    InvalidLanguageResponse(#[from] std::str::Utf8Error),
}

impl Error {
    pub(crate) unsafe fn from_ret(ret: i32) -> Self {
        debug_assert_ne!(ret, 0, "success code passed to Error::from_ret");
        Error::OutOfMemory(ret)
    }
}
