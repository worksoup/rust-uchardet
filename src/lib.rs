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

#[cfg(feature = "auto_encoding_reader")]
pub mod auto_encoding_reader;
mod candidates;
mod detector;
#[cfg(feature = "encoding")]
pub mod encoding;
mod error;

extern crate uchardet_git_sys as sys;

pub use candidates::*;
pub use detector::*;
pub use error::*;

#[cfg(feature = "encoding")]
pub fn detect_encoding(data: impl AsRef<[u8]>) -> Result<&'static encoding_rs::Encoding, Error> {
    let candidates = UCharsetDetector::detect_data(data)?;
    candidates
        .best()
        .ok_or(Error::UnrecognizableCharset)?
        .encoding()
}

pub fn detect_encoding_name(data: impl AsRef<[u8]>) -> Result<String, Error> {
    let candidates = UCharsetDetector::detect_data(data)?;
    candidates
        .best()
        .ok_or(Error::UnrecognizableCharset)?
        .encoding_name()
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    fn assert_detected_encoding(data: &[u8], expected: &str) {
        let encoding =
            crate::detect_encoding_name(data).expect("should have at least one candidate");
        assert_eq!(encoding, expected);
    }

    #[test]
    fn test_detect_encoding_ascii() {
        assert_detected_encoding(b"ascii", "ASCII");
    }

    #[test]
    fn test_detect_encoding_utf8() {
        assert_detected_encoding("©français".as_bytes(), "UTF-8");
    }

    #[test]
    fn test_detect_encoding_windows1252() {
        let data = &[
            0x46, 0x93, 0x72, 0x61, 0x6e, 0xe7, 0x6f, 0x69, 0x73, 0xe9, 0x94,
        ];
        assert_detected_encoding(data, "WINDOWS-1252");
    }
}
