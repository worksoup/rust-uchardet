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

pub fn to_standard(encoding: &str) -> Option<&'static encoding_rs::Encoding> {
    if encoding.eq_ignore_ascii_case("MAC-CYRILLIC") {
        return Some(encoding_rs::X_MAC_CYRILLIC);
    }
    if encoding.eq_ignore_ascii_case("HZ-GB-2312")
        || encoding.eq_ignore_ascii_case("ISO-2022-CN")
        || encoding.eq_ignore_ascii_case("ISO-2022-KR")
    {
        return None;
    }
    encoding_rs::Encoding::for_label_no_replacement(encoding.as_bytes())
}

#[cfg(test)]
mod tests {

    macro_rules! assert_encoding_for_name {
        ($encoding:ident, $name:expr) => {{
            let expected = encoding_rs::$encoding;
            let actual =
                encoding_rs::Encoding::for_label($name.as_bytes()).expect("Expected an encoding");
            assert_eq!(expected.name(), actual.name());
        }};
    }

    macro_rules! assert_no_encoding_for_name {
        ($name:expr) => {{
            assert_eq!(encoding_rs::Encoding::for_label($name.as_bytes()), None);
        }};
    }

    #[test]
    fn test_encoding_for_label_handles_uchardet_names_as_expected() {
        // Note that we may not want to map "ASCII" to 1252 at higher levels.
        assert_encoding_for_name!(WINDOWS_1252, "ASCII");
        assert_encoding_for_name!(BIG5, "BIG5");
        assert_encoding_for_name!(EUC_JP, "EUC-JP");
        assert_encoding_for_name!(EUC_KR, "EUC-KR");
        assert_encoding_for_name!(GB18030, "GB18030");
        assert_encoding_for_name!(IBM866, "IBM866");
        assert_encoding_for_name!(ISO_2022_JP, "ISO-2022-JP");
        assert_encoding_for_name!(WINDOWS_1252, "ISO-8859-1"); // 浏览器应按 ‌Windows-1252‌ 解析，需确认 uchardet 是否也是一样的处理。
        assert_encoding_for_name!(ISO_8859_2, "ISO-8859-2");
        assert_encoding_for_name!(ISO_8859_3, "ISO-8859-3");
        assert_encoding_for_name!(ISO_8859_4, "ISO-8859-4");
        assert_encoding_for_name!(ISO_8859_5, "ISO-8859-5");
        assert_encoding_for_name!(ISO_8859_6, "ISO-8859-6");
        assert_encoding_for_name!(ISO_8859_7, "ISO-8859-7");
        assert_encoding_for_name!(ISO_8859_8, "ISO-8859-8");
        assert_encoding_for_name!(WINDOWS_1254, "ISO-8859-9"); // 需确认。
        assert_encoding_for_name!(ISO_8859_10, "ISO-8859-10");
        assert_encoding_for_name!(WINDOWS_874, "ISO-8859-11");
        assert_encoding_for_name!(ISO_8859_13, "ISO-8859-13");
        assert_encoding_for_name!(ISO_8859_15, "ISO-8859-15");
        assert_encoding_for_name!(ISO_8859_16, "ISO-8859-16");
        assert_encoding_for_name!(WINDOWS_874, "TIS-620");
        assert_encoding_for_name!(KOI8_R, "KOI8-R");
        assert_encoding_for_name!(SHIFT_JIS, "SHIFT_JIS");
        assert_encoding_for_name!(UTF_8, "UTF-8");
        // This maps to UTF_16LE because that's the most common, but the
        // decoder can actually decode either in the default mode by detecting
        // byte order marks, which is the only way `uchardet` can detect
        // either, so it's not a problem.
        assert_encoding_for_name!(UTF_16LE, "UTF-16");
        // This is not supported by the encoding standard, because it appears
        // to be rare in the wild.
        assert_encoding_for_name!(WINDOWS_1250, "Windows-1250");
        assert_encoding_for_name!(WINDOWS_1251, "Windows-1251");
        assert_encoding_for_name!(WINDOWS_1252, "Windows-1252");
        assert_encoding_for_name!(WINDOWS_1253, "Windows-1253");
        assert_encoding_for_name!(WINDOWS_1255, "Windows-1255");
        assert_encoding_for_name!(WINDOWS_1256, "Windows-1256");
        assert_encoding_for_name!(WINDOWS_1257, "Windows-1257");
        assert_encoding_for_name!(WINDOWS_1258, "Windows-1258");

        // X-*
        assert_no_encoding_for_name!("MAC-CENTRALEUROPE");
        assert_no_encoding_for_name!("MAC-CYRILLIC");
        // X-*
        assert_no_encoding_for_name!("X-MAC-CENTRALEUROPE");
        assert_encoding_for_name!(X_MAC_CYRILLIC, "X-MAC-CYRILLIC");

        // REPLACEMENT
        assert_encoding_for_name!(REPLACEMENT, "HZ-GB-2312");
        assert_encoding_for_name!(REPLACEMENT, "ISO-2022-CN");
        assert_encoding_for_name!(REPLACEMENT, "ISO-2022-KR");

        assert_no_encoding_for_name!("IBM737");
        assert_no_encoding_for_name!("CP737");
        // This does not appear to be supported by the Encoding Standard.
        assert_no_encoding_for_name!("EUC-TW");
        assert_no_encoding_for_name!("GEORGIAN-ACADEMY");
        assert_no_encoding_for_name!("GEORGIAN-PS");
        assert_no_encoding_for_name!("IBM852");
        assert_no_encoding_for_name!("IBM855");
        assert_no_encoding_for_name!("IBM862");
        assert_no_encoding_for_name!("IBM865");
        assert_no_encoding_for_name!("Johab");
        // superset of EUC-KR.
        assert_no_encoding_for_name!("UHC");
        assert_no_encoding_for_name!("UTF-32BE");
        assert_no_encoding_for_name!("UTF-32LE");
        assert_no_encoding_for_name!("VISCII");
        assert_no_encoding_for_name!("X-ISO-10646-UCS-4-34121");
        assert_no_encoding_for_name!("X-ISO-10646-UCS-4-21431");
    }
}
