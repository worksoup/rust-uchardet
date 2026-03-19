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
}

/// 自动检测和转换文本编码的读取器
///
/// 该读取器使用 `uchardet` 自动检测输入流的字符编码，并将其实时转换为 UTF-8 编码。
/// 支持多种常见编码格式，包括 GB18030、GBK、BIG5 等。
pub struct AutoEncodingReader<R: Read> {
    /// 底层原始读取器
    reader: R,
    /// 从底层读取器读取数据的缓冲区（用于检测和后续读取）
    buffer: Box<[u8; 8 * 1024]>,
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
        self.decoder = self.decoder.encoding().new_decoder_without_bom_handling();
        self.had_replacement_or_cant_map = false;
        self.transcode_done = false;
        self.eof = false;
        Ok(())
    }
}

impl<R: Read> AutoEncodingReader<R> {
    /// 使用已确定的编码创建读取器（内部使用）
    fn new_with_decoder(
        reader: R,
        decoder: Decoder,
        mut initial_data: Vec<u8>,
        mut decoded_data: Vec<u8>,
    ) -> Self {
        let no_transcoding_needed = decoder.encoding().name() == "UTF-8";
        if no_transcoding_needed {
            // 若包含 BOM，跳过 BOM
            if initial_data
                .windows(3)
                .next()
                .is_some_and(|maybe_bom| maybe_bom == b"\xef\xbb\xbf")
            {
                initial_data.drain(..3);
            }
            if decoded_data.is_empty() {
                (initial_data, decoded_data) = (decoded_data, initial_data)
            } else {
                decoded_data.append(&mut initial_data);
            }
        }
        Self {
            reader,
            buffer: Box::new([0u8; 8 * 1024]),
            read_buffer: initial_data,
            write_buffer: decoded_data,
            decoder,
            had_replacement_or_cant_map: false,
            transcode_done: false,
            eof: false,
            no_transcoding_needed,
        }
    }

    /// 使用编码检测和后备编码列表创建新的读取器
    ///
    /// 该方法会读取前 8KB 数据用于编码检测，如果无法检测则尝试后备编码。
    ///
    /// # 参数
    /// - `reader`: 底层字节流读取器
    /// - `fallbacks`: 编码检测失败时的后备编码列表（按优先级顺序）
    ///
    /// # 错误
    /// 当无法检测到合适的编码时会返回 `EncodingError::CharsetError`
    pub fn new_with_fallbacks(
        mut reader: R,
        fallbacks: &[&'static encoding_rs::Encoding],
    ) -> Result<Self, EncodingError> {
        // 读取 8KB 数据用于检测
        let mut buf = vec![0u8; 8 * 1024];
        let n = reader.read(&mut buf)?;
        let eof = n < buf.len();
        buf.truncate(n);

        if n == 0 {
            // 空文件，直接返回 UTF-8 解码器
            let decoder = encoding_rs::UTF_8.new_decoder_without_bom_handling();
            return Ok(Self::new_with_decoder(reader, decoder, buf, vec![]));
        }

        // 使用 uchardet 检测编码
        let candidates = CharsetDetector::detect_data(&buf)?;
        let best_candidate = candidates.best();

        if let Some(candidate) = best_candidate {
            let name = candidate.encoding_name()?;
            // 将 uchardet 名称映射为 encoding_rs::Encoding
            let encoding = crate::encoding::to_standard(name)
                .or_else(|| encoding_rs::Encoding::for_label(name.as_bytes()));
            if let Some(enc) = encoding {
                let decoder = enc.new_decoder_without_bom_handling();
                return Ok(Self::new_with_decoder(reader, decoder, buf, vec![]));
            } else {
                // 映射失败，回退到后备编码
            }
        }
        if eof {
            let mut buf_ = Vec::new();
            // 编码检测失败且已到达文件末尾，尝试后备编码
            for &fallback in fallbacks {
                let mut reader_ = AutoEncodingReader::new_with_decoder(
                    &*buf,
                    fallback.new_decoder(),
                    vec![],
                    Vec::with_capacity(5 * 512),
                );
                buf_.clear();
                match reader_.read_to_end(&mut buf_) {
                    Ok(_) => {
                        return Ok(AutoEncodingReader::new_with_decoder(
                            reader,
                            reader_.decoder,
                            vec![],
                            buf_,
                        ));
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }
        }

        // 所有后备编码都尝试失败
        Err(EncodingError::CharsetError(
            "未能检测到合适的字符编码，且所有后备编码均失败。".to_owned(),
        ))
    }

    /// 使用默认后备编码列表创建新的读取器
    ///
    /// 默认后备编码：GB18030, GBK, UTF_8, BIG5（针对中文环境）
    #[inline]
    pub fn new(reader: R) -> Result<Self, EncodingError> {
        let fallbacks = [encoding_rs::GB18030, encoding_rs::GBK, encoding_rs::BIG5];
        Self::new_with_fallbacks(reader, &fallbacks)
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

    pub fn decoder(&self) -> &Decoder {
        &self.decoder
    }

    pub fn encoding(&self) -> &'static encoding_rs::Encoding {
        self.decoder().encoding()
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
        let n = self.reader.read(self.buffer.as_mut_slice())?;
        self.read_buffer.extend_from_slice(&self.buffer[..n]);
        self.eof = n == 0;
        let num_written = self.decode(buffer);
        Ok(num_written)
    }
}
