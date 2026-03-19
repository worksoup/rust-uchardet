use crate::{CharsetDetector, Error as DetectorError};
use encoding_rs::Decoder;
use reader_ext::Rewind;
use std::io::{Read, Seek};

/// 自动编码检测和转换读取器的错误类型
#[derive(Debug, thiserror::Error)]
pub enum EncodingError {
    /// 底层IO操作错误
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    /// 字符编码检测或转换错误
    #[error("字符编码错误：{0}")]
    CharsetError(String),

    /// uchardet 库错误
    #[error(transparent)]
    DetectorError(#[from] DetectorError),

    #[error("未设置底层读取器")]
    NoReader,
}

/// 自动检测和转换文本编码的读取器
///
/// 该读取器使用 `uchardet` 自动检测输入流的字符编码，并将其实时转换为 UTF-8 编码。
/// 支持多种常见编码格式，包括 GB18030、GBK、BIG5 等。
pub struct AutoEncodingReader<R: Read> {
    /// 底层原始读取器
    reader: R,
    /// 从底层读取器读取数据的可变大小的读取缓冲区（用于检测和后续读取）
    buffer: Box<[u8]>,
    /// 已读取但尚未解码的原始字节缓冲区（包括用于检测的部分）
    read_buffer: Vec<u8>,
    /// 已解码为 UTF-8 的输出缓冲区
    write_buffer: Vec<u8>,
    /// 解码器实例，用于将原始编码转换为 UTF-8
    decoder: Decoder,
    /// 标记解码过程中是否出现无法映射的字符（使用了替换字符）
    had_replacement_or_cant_map: bool,
    /// 标记转码是否已完成（所有输入已处理）
    transcode_done: bool,
    /// 标记是否已到达输入流的末尾
    eof: bool,
    /// 标记是否无需转码（输入已经是 UTF-8 编码）
    no_transcoding_needed: bool,
}

impl<R: Read + Seek> Rewind for AutoEncodingReader<R> {
    fn try_rewind(&mut self) -> std::io::Result<()> {
        self.reader.rewind()?;
        self.read_buffer.clear();
        self.write_buffer.clear();
        // 重置解码器为初始状态
        self.decoder = self.decoder.encoding().new_decoder();
        self.had_replacement_or_cant_map = false;
        self.transcode_done = false;
        self.eof = false;
        Ok(())
    }
}

impl<R: Read> AutoEncodingReader<R> {
    /// 内部构造：使用已知解码器和初始数据创建读取器
    pub(crate) fn new_with_decoder(
        reader: R,
        decoder: Decoder,
        initial_data: Vec<u8>,
        decoded_data: Vec<u8>,
        read_buffer_size: usize,
    ) -> Self {
        let no_transcoding_needed = decoder.encoding().name() == "UTF-8";
        let (mut initial_data, mut decoded_data) = (initial_data, decoded_data);
        if no_transcoding_needed {
            // 跳过 BOM
            if initial_data
                .windows(3)
                .next()
                .is_some_and(|maybe_bom| maybe_bom == b"\xef\xbb\xbf")
            {
                initial_data.drain(..3);
            }
            if decoded_data.is_empty() {
                (initial_data, decoded_data) = (decoded_data, initial_data);
            } else {
                decoded_data.append(&mut initial_data);
            }
        }
        // 分配指定大小的读取缓冲区
        let buffer = vec![0u8; read_buffer_size].into_boxed_slice();
        Self {
            reader,
            buffer,
            read_buffer: initial_data,
            write_buffer: decoded_data,
            decoder,
            had_replacement_or_cant_map: false,
            transcode_done: false,
            eof: false,
            no_transcoding_needed,
        }
    }

    /// 使用检测缓冲区大小、读取缓冲区大小和后备编码列表创建读取器
    ///
    /// 该方法会读取检测缓冲区大小的数据用于编码检测，如果无法检测则尝试后备编码。
    ///
    /// # 参数
    /// - `reader`: 底层字节流读取器
    /// - `fallbacks`: 编码检测失败时的后备编码列表（按优先级顺序）
    /// - `detect_buffer_size`: 用于编码检测的初始读取字节数
    /// - `read_buffer_size`: 后续读取时使用的内部缓冲区大小
    ///
    /// # 错误
    /// 当无法检测到合适的编码时会返回 `EncodingError::CharsetError`
    pub fn new_with_fallbacks(
        reader: R,
        fallbacks: &[&'static encoding_rs::Encoding],
        detect_buffer_size: usize,
        read_buffer_size: usize,
    ) -> Result<Self, EncodingError> {
        AutoEncodingReaderBuilder::new()
            .reader(reader)
            .fallbacks(fallbacks)
            .detect_buffer_size(detect_buffer_size)
            .read_buffer_size(read_buffer_size)
            .build()
    }

    /// 简便方法：使用默认检测缓冲区大小 (8KB) 和读取缓冲区大小 (8KB)
    #[inline]
    pub fn new_with_fallbacks_default(
        reader: R,
        fallbacks: &[&'static encoding_rs::Encoding],
    ) -> Result<Self, EncodingError> {
        Self::new_with_fallbacks(reader, fallbacks, 8192, 8192)
    }

    /// 使用默认检测缓冲区大小 (8KB) 和读取缓冲区大小 (8KB)、默认后备编码列表创建新的读取器
    ///
    /// 默认后备编码：GB18030, GBK, BIG5（针对中文环境）
    #[inline]
    pub fn new(reader: R) -> Result<Self, EncodingError> {
        let fallbacks = [encoding_rs::GB18030, encoding_rs::GBK, encoding_rs::BIG5];
        Self::new_with_fallbacks_default(reader, &fallbacks)
    }

    /// 从输出缓冲区复制数据到用户提供的缓冲区
    fn copy_from_write_buffer_to(&mut self, buffer: &mut [u8]) -> usize {
        let min = std::cmp::min(buffer.len(), self.write_buffer.len());
        buffer[..min].copy_from_slice(&self.write_buffer[..min]);
        self.write_buffer = self.write_buffer[min..].to_vec();
        min
    }

    /// 解码原始字节为 UTF-8
    ///
    /// 将 `read_buffer` 中的原始字节解码为 UTF-8，写入到用户缓冲区或内部缓冲区
    fn decode(&mut self, buffer: &mut [u8]) -> usize {
        if self.read_buffer.is_empty() && !self.eof {
            return 0;
        }

        if buffer.len() > 1024 {
            // 用户缓冲区足够大，直接解码到其中
            let (coder_result, num_read, num_written, has_replacement) = self
                .decoder
                .decode_to_utf8(&self.read_buffer, buffer, self.eof);
            self.read_buffer = self.read_buffer[num_read..].to_vec();
            self.had_replacement_or_cant_map |= has_replacement;
            self.transcode_done =
                (coder_result == encoding_rs::CoderResult::InputEmpty) && self.eof;
            return num_written;
        }

        // 用户缓冲区太小，解码到内部缓冲区
        self.write_buffer.clear();
        self.write_buffer.resize(8 * 1024, 0);
        let (coder_result, num_read, num_written, has_replacement) =
            self.decoder
                .decode_to_utf8(&self.read_buffer, &mut self.write_buffer, self.eof);
        self.read_buffer = self.read_buffer[num_read..].to_vec();
        self.write_buffer.truncate(num_written);
        self.had_replacement_or_cant_map |= has_replacement;
        self.transcode_done = (coder_result == encoding_rs::CoderResult::InputEmpty) && self.eof;
        if num_written > 0 {
            return self.copy_from_write_buffer_to(buffer);
        }
        0
    }

    /// 检查解码过程中是否出现了无法映射的字符
    ///
    /// 返回 `true` 表示解码过程中使用了替换字符（通常为 �）
    pub fn had_replacement_or_cant_map(&self) -> bool {
        self.had_replacement_or_cant_map
    }
    /// 返回当前使用的解码器
    pub fn decoder(&self) -> &Decoder {
        &self.decoder
    }

    /// 返回当前使用的编码
    pub fn encoding(&self) -> &'static encoding_rs::Encoding {
        self.decoder.encoding()
    }
}

impl<R: Read> Read for AutoEncodingReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }

        // 优先从输出缓冲区取数据
        if !self.write_buffer.is_empty() {
            return Ok(self.copy_from_write_buffer_to(buffer));
        }

        // 如果无需转码（已经是 UTF-8），直接传递数据
        if self.no_transcoding_needed {
            // 如果 read_buffer 还有未读取的数据（例如 BOM 之后的），应先处理；
            // 但实际上此时永远为空（构造时已直接写入 write_buffer, 后续也不再写入 read_buffer）。
            // if !self.read_buffer.is_empty() {
            //     let n = std::cmp::min(buffer.len(), self.read_buffer.len());
            //     buffer[..n].copy_from_slice(&self.read_buffer[..n]);
            //     self.read_buffer = self.read_buffer[n..].to_vec();
            //     return Ok(n);
            // }
            let n = self.reader.read(buffer)?;
            return Ok(n);
        }

        // 如果转码已完成，返回 0 表示 EOF
        if self.transcode_done {
            return Ok(0);
        }

        // 如果 read_buffer 有数据，尝试解码
        if !self.read_buffer.is_empty() {
            let num_written = self.decode(buffer);
            if num_written > 0 {
                return Ok(num_written);
            }
        }

        // 从底层读取器读取更多数据
        let n = self.reader.read(self.buffer.as_mut())?;
        self.read_buffer.extend_from_slice(&self.buffer[..n]);
        self.eof = n == 0;
        let num_written = self.decode(buffer);
        Ok(num_written)
    }
}

/// 构建器，用于配置 AutoEncodingReader
pub struct AutoEncodingReaderBuilder<R> {
    reader: Option<R>,
    fallbacks: Vec<&'static encoding_rs::Encoding>,
    detect_buffer_size: usize,
    read_buffer_size: usize,
    language_weights: Vec<(String, f32)>,
    default_weight: Option<f32>,
}

impl<R: Read> AutoEncodingReaderBuilder<R> {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            reader: None,
            fallbacks: Vec::new(),
            detect_buffer_size: 8192,
            read_buffer_size: 8192,
            language_weights: Vec::new(),
            default_weight: None,
        }
    }

    /// 设置底层读取器（必需）
    pub fn reader(mut self, reader: R) -> Self {
        self.reader = Some(reader);
        self
    }

    /// 设置后备编码列表
    pub fn fallbacks(mut self, fallbacks: &[&'static encoding_rs::Encoding]) -> Self {
        self.fallbacks = fallbacks.to_vec();
        self
    }

    /// 设置检测缓冲区大小（字节数）
    pub fn detect_buffer_size(mut self, size: usize) -> Self {
        self.detect_buffer_size = size;
        self
    }

    /// 设置后续读取缓冲区大小（字节数）
    pub fn read_buffer_size(mut self, size: usize) -> Self {
        self.read_buffer_size = size;
        self
    }

    /// 添加语言权重（可多次调用）
    pub fn language_weight(mut self, language: &str, weight: f32) -> Self {
        self.language_weights.push((language.to_owned(), weight));
        self
    }

    /// 设置默认权重
    pub fn default_weight(mut self, weight: f32) -> Self {
        self.default_weight = Some(weight);
        self
    }

    /// 构建 AutoEncodingReader
    pub fn build(self) -> Result<AutoEncodingReader<R>, EncodingError> {
        let mut reader = self.reader.ok_or(EncodingError::NoReader)?;

        // 读取 detect_buffer_size 字节用于检测
        let mut buf = vec![0u8; self.detect_buffer_size];
        let n = reader.read(&mut buf)?;
        let eof = n < buf.len();
        buf.truncate(n);

        if n == 0 {
            // 空文件，直接返回 UTF-8 解码器
            let decoder = encoding_rs::UTF_8.new_decoder_without_bom_handling();
            return Ok(AutoEncodingReader::new_with_decoder(
                reader,
                decoder,
                buf,
                vec![],
                self.read_buffer_size,
            ));
        }

        // 使用 uchardet 检测编码
        let mut detector = CharsetDetector::new();
        for (lang, weight) in &self.language_weights {
            detector.weigh_language(lang, *weight)?;
        }
        if let Some(w) = self.default_weight {
            detector.set_default_weight(w);
        }

        detector.feed_data(&buf)?;
        let candidates = detector.detect();
        let best_candidate = candidates.best();

        if let Some(candidate) = best_candidate {
            let name = candidate.encoding_name()?;
            let encoding = crate::encoding::to_standard(name)
                .or_else(|| encoding_rs::Encoding::for_label(name.as_bytes()));
            if let Some(enc) = encoding {
                let decoder = enc.new_decoder();
                return Ok(AutoEncodingReader::new_with_decoder(
                    reader,
                    decoder,
                    buf,
                    vec![],
                    self.read_buffer_size,
                ));
            }
        }

        // 检测失败或编码名不支持，尝试后备编码
        if eof {
            let mut decoded = Vec::new();
            for &fallback in &self.fallbacks {
                let mut tmp_reader = AutoEncodingReader::new_with_decoder(
                    &*buf,
                    fallback.new_decoder(),
                    vec![],
                    Vec::with_capacity(5 * 512),
                    self.read_buffer_size,
                );
                decoded.clear();
                if tmp_reader.read_to_end(&mut decoded).is_ok() {
                    return Ok(AutoEncodingReader::new_with_decoder(
                        reader,
                        tmp_reader.decoder,
                        vec![],
                        decoded,
                        self.read_buffer_size,
                    ));
                }
            }
        }

        Err(EncodingError::CharsetError(
            "未能检测到合适的字符编码，且所有后备编码均失败。".to_owned(),
        ))
    }
}

impl<R: Read> Default for AutoEncodingReaderBuilder<R> {
    fn default() -> Self {
        Self::new()
    }
}
