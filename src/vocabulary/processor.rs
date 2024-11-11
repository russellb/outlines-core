use std::collections::HashMap;

use once_cell::sync::Lazy;
use serde::Deserialize;
use tokenizers::normalizers::Replace;
use tokenizers::{DecoderWrapper, Tokenizer};

use crate::TokenProcessorError;

pub type Result<T, E = TokenProcessorError> = std::result::Result<T, E>;

/// GPT2-like tokenizers have multibyte tokens that can have a mix of full and incomplete
/// utf-8 characters. For example, b` \xf0` can be one token. These tokenizers map each
/// byte to a valid UTF-8 character. And we need to map back those characters into bytes.
///
/// "ĠO" = [U+0120, U+004F] should be interpreted as [0x20, 0x4F] = " O"
/// or
/// "Ġal" = [U+0120, U+0061, U+006C] should be interpreted as [0x20, 0x61, 0x6C] = " al"
///
/// We'll use the following the mapping for this transition:
/// ---
/// 'Ā' == '\u{0100}' -> 0x00 == 0
/// 'ā' == '\u{0101}' -> 0x01 == 1
/// 'Ă' == '\u{0102}' -> 0x02 == 2
/// ...
/// 'Ğ' == '\u{011E}' -> 0x1E == 30
/// 'ğ' == '\u{011F}' -> 0x1F == 31
/// 'Ġ' == '\u{0120}' -> 0x20 == 32
/// ---
/// '!' == '\u{0021}' -> 0x21 == 33
/// '"' == '\u{0022}' -> 0x22 == 34
/// '#' == '\u{0023}' -> 0x23 == 35
/// ...
/// '|' == '\u{007C}' -> 0x7C == 124
/// '}' == '\u{007D}' -> 0x7D == 125
/// '~' == '\u{007E}' -> 0x7E == 126
/// ---
/// 'ġ' == '\u{0121}' -> 0x7F == 127
/// 'Ģ' == '\u{0122}' -> 0x80 == 128
/// 'ģ' == '\u{0123}' -> 0x81 == 129
/// ...
/// 'ŀ' == '\u{0140}' -> 0x9E == 158
/// 'Ł' == '\u{0141}' -> 0x9F == 159
/// 'ł' == '\u{0142}' -> 0xA0 == 160
/// ---
/// '¡' == '\u{00A1}' -> 0xA1 == 161
/// '¢' == '\u{00A2}' -> 0xA2 == 162
/// '£' == '\u{00A3}' -> 0xA3 == 163
/// ...
/// 'ª' == '\u{00AA}' -> 0xAA == 170
/// '«' == '\u{00AB}' -> 0xAB == 171
/// '¬' == '\u{00AC}' -> 0xAC == 172
/// ---
/// 'Ń' == '\u{0143}' -> 0xAD == 173
/// ---
/// '®' == '\u{00AE}' -> 0xAE == 174
/// '¯' == '\u{00AF}' -> 0xAF == 175
/// '°' == '\u{00B0}' -> 0xB0 == 176
/// ...
/// 'ý' == '\u{00FD}' -> 0xFD == 253
/// 'þ' == '\u{00FE}' -> 0xFE == 254
/// 'ÿ' == '\u{00FF}' -> 0xFF == 255
/// ---
static CHAR_MAP: Lazy<HashMap<char, u8>> = Lazy::new(|| {
    let mut char_map = HashMap::with_capacity(256);
    let mut key = 0x100u32;
    for byte in 0..=255u8 {
        let char = byte as char;
        if matches!(
            char, '!'..='~' | '\u{00A1}'..='\u{00AC}' | '\u{00AE}'..='\u{00FF}',
        ) {
            char_map.insert(char, byte);
        } else if let Some(ch) = char::from_u32(key) {
            char_map.insert(ch, byte);
            key += 1;
        }
    }
    char_map
});

/// Token processor to adjust tokens according to the tokenizer's level.
#[derive(Debug)]
pub(crate) struct TokenProcessor {
    level: TokenProcessorLevel,
}

/// Recognized tokenizer's levels.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenProcessorLevel {
    /// Matches byte level tokenizer (e.g., gpt2).
    Byte,
    /// Matches byte fallback tokenizer (e.g., llama), which have <0x__> tokens for
    /// all __ >= 0x80 to represent incomplete UTF-8 sequences.
    ByteFallback(Mods),
}

impl std::fmt::Display for TokenProcessorLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Byte => write!(f, "Byte Level"),
            Self::ByteFallback(mods) => write!(f, "Byte Fallback Level with mods: {:?}", mods),
        }
    }
}

/// Modifications to be applied by `ByteFallback` `TokenProcessorLevel`.
#[derive(Debug, Clone, PartialEq)]
pub struct Mods {
    spacechar: char,
}

/// Default string modification to be applied by `TokenProcessor` of `ByteFallback` level.
static DEFAULT_MODS: Mods = Mods { spacechar: ' ' };

impl Mods {
    /// Apply default modifications.
    fn apply_default(&self, token: String) -> String {
        let to = DEFAULT_MODS.spacechar.to_string();
        token.replace(self.spacechar, &to)
    }
}

#[derive(Debug, Deserialize)]
struct ReplaceDecoder {
    content: String,
    pattern: ReplacePattern,
}

impl ReplaceDecoder {
    fn space_replacement(&self) -> Option<char> {
        if self.content != " " {
            return None;
        }
        match &self.pattern {
            ReplacePattern::String(pattern) => {
                let mut chars = pattern.chars();
                let char = chars.next();
                if let Some(replacement) = char {
                    if chars.next().is_none() {
                        return Some(replacement);
                    }
                }
                None
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum ReplacePattern {
    String(String),
}

impl TokenProcessor {
    /// Create new `TokenProcessor` with the level defined based on tokenizer's decoders.
    pub(crate) fn new(tokenizer: &Tokenizer) -> Result<Self> {
        match tokenizer.get_decoder() {
            None => Err(TokenProcessorError::UnsupportedTokenizer),
            Some(decoder) => match decoder {
                DecoderWrapper::ByteLevel(_) => Ok(Self {
                    level: TokenProcessorLevel::Byte,
                }),
                DecoderWrapper::Sequence(decoding_sequence) => {
                    let mut is_byte_fallback = false;
                    let mut spacechar = ' ';

                    for decoder in decoding_sequence.get_decoders() {
                        match decoder {
                            DecoderWrapper::ByteFallback(_) => {
                                is_byte_fallback = true;
                            }
                            DecoderWrapper::Replace(replace) => {
                                // `Replace` decoder would replace a pattern in the output with something else,
                                // which we need to know.
                                let decoder = Self::unpack_decoder(replace)?;
                                if let Some(replacement) = decoder.space_replacement() {
                                    spacechar = replacement;
                                }
                            }
                            _ => {}
                        }
                    }

                    if is_byte_fallback {
                        Ok(Self {
                            level: TokenProcessorLevel::ByteFallback(Mods { spacechar }),
                        })
                    } else {
                        Err(TokenProcessorError::UnsupportedTokenizer)
                    }
                }
                _ => Err(TokenProcessorError::UnsupportedTokenizer),
            },
        }
    }

    /// Process each token based on the level of `TokenProcessor`.
    pub(crate) fn process(&self, token: String) -> Result<Vec<u8>> {
        match &self.level {
            TokenProcessorLevel::Byte => {
                let mut bytes = vec![];
                for char in token.chars() {
                    match CHAR_MAP.get(&char) {
                        None => return Err(TokenProcessorError::ByteProcessorFailed),
                        Some(b) => bytes.push(*b),
                    }
                }
                Ok(bytes)
            }
            TokenProcessorLevel::ByteFallback(mods) => {
                // If the token is of form `<0x__>`:
                if token.len() == 6 && token.starts_with("<0x") && token.ends_with('>') {
                    // Get to a single byte specified in the __ part and parse it in base 16 to a byte.
                    match u8::from_str_radix(&token[3..5], 16) {
                        Ok(byte) => Ok([byte].to_vec()),
                        Err(_) => Err(TokenProcessorError::ByteFallbackProcessorFailed),
                    }
                } else {
                    Ok(mods.apply_default(token).as_bytes().to_vec())
                }
            }
        }
    }

    /// Since all fields of `Replace` are private with no getters, we'll have to unpack it into our own.
    fn unpack_decoder(decoder: &Replace) -> Result<ReplaceDecoder> {
        match serde_json::to_value(decoder) {
            Err(_) => Err(TokenProcessorError::DecoderUnpackingFailed),
            Ok(value) => match serde_json::from_value(value) {
                Ok(d) => Ok(d),
                Err(_) => Err(TokenProcessorError::DecoderUnpackingFailed),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_level_processor() {
        let model = "openai-community/gpt2";
        let tokenizer = Tokenizer::from_pretrained(model, None).expect("Tokenizer failed");
        let processor = TokenProcessor::new(&tokenizer).expect("Processor failed");

        assert_eq!(processor.level, TokenProcessorLevel::Byte);

        for (ch, byte) in [
            ('Ā', 0x00),
            ('ā', 0x01),
            ('Ă', 0x02),
            ('Ğ', 0x1E),
            ('ğ', 0x1F),
            ('Ġ', 0x20),
            ('!', 0x21),
            ('"', 0x22),
            ('#', 0x23),
            ('|', 0x7C),
            ('}', 0x7D),
            ('~', 0x7E),
            ('ġ', 0x7F),
            ('Ģ', 0x80),
            ('ģ', 0x81),
            ('ŀ', 0x9E),
            ('Ł', 0x9F),
            ('ł', 0xA0),
            ('¡', 0xA1),
            ('¢', 0xA2),
            ('£', 0xA3),
            ('ª', 0xAA),
            ('«', 0xAB),
            ('¬', 0xAC),
            ('Ń', 0xAD),
            ('®', 0xAE),
            ('¯', 0xAF),
            ('°', 0xB0),
            ('ý', 0xFD),
            ('þ', 0xFE),
            ('ÿ', 0xFF),
        ] {
            let processed = processor.process(ch.to_string()).expect("Not processed");
            assert_eq!(processed, [byte]);
        }
    }

    #[test]
    fn byte_fallback_level_processor() {
        let model = "hf-internal-testing/llama-tokenizer";
        let tokenizer = Tokenizer::from_pretrained(model, None).expect("Tokenizer failed");
        let processor = TokenProcessor::new(&tokenizer).expect("Processor failed");
        let spacechar = '▁';
        let mods = Mods { spacechar };

        assert_eq!(processor.level, TokenProcessorLevel::ByteFallback(mods));

        for (input, expected) in [
            ("abc", vec![0x61, 0x62, 0x63]),
            ("<0x61>", vec![0x61]),
            ("<0x61>a", vec![0x3C, 0x30, 0x78, 0x36, 0x31, 0x3E, 0x61]),
            (&spacechar.to_string(), vec![0x20]),
            (
                &format!("{}{}abc", spacechar, spacechar),
                vec![0x20, 0x20, 0x61, 0x62, 0x63],
            ),
            (
                &format!("{}{}{}", spacechar, spacechar, spacechar),
                vec![0x20, 0x20, 0x20],
            ),
        ] {
            let processed = processor.process(input.to_string()).expect("Not processed");
            assert_eq!(processed, expected);
        }
    }

    #[test]
    fn unsupported_tokenizer_error() {
        let model = "hf-internal-testing/tiny-random-XLMRobertaXLForCausalLM";
        let tokenizer = Tokenizer::from_pretrained(model, None).expect("Tokenizer failed");

        let result = TokenProcessor::new(&tokenizer);
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e, TokenProcessorError::UnsupportedTokenizer)
        }
    }

    #[test]
    fn byte_processor_error() {
        let model = "openai-community/gpt2";
        let tokenizer = Tokenizer::from_pretrained(model, None).expect("Tokenizer failed");
        let processor = TokenProcessor::new(&tokenizer).expect("Processor failed");

        for token in ["𝒜𝒷𝒸𝒟𝓔", "🦄🌈🌍🔥🎉", "京东购物"] {
            let result = processor.process(token.to_string());
            assert!(result.is_err());
            if let Err(e) = result {
                assert_eq!(e, TokenProcessorError::ByteProcessorFailed)
            }
        }
    }

    #[test]
    fn byte_fallback_processor_error() {
        let model = "hf-internal-testing/llama-tokenizer";
        let tokenizer = Tokenizer::from_pretrained(model, None).expect("Tokenizer failed");
        let processor = TokenProcessor::new(&tokenizer).expect("Processor failed");

        let result = processor.process("<0x6y>".to_string());
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e, TokenProcessorError::ByteFallbackProcessorFailed)
        }
    }
}
