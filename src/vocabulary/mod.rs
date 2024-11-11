use std::collections::HashMap;

use tokenizers::normalizers::Sequence;
use tokenizers::{FromPretrainedParameters, NormalizerWrapper, Tokenizer};

use crate::{prelude::*, TokenizerError, VocabularyError};

use locator::EosTokenLocator;
use processor::TokenProcessor;

mod locator;
mod processor;

/// Vocabulary of an LLM.
///
/// ## Examples
///
/// ```rust
/// # use outlines_core::prelude::*;
/// #
/// let vocabulary = Vocabulary::new(None)
///     .insert("blah", 0)
///     .insert("1a", 1)
///     .insert("2", 2)
///     .insert("0", 3);
/// ```
#[derive(Clone, Debug, Default)]
pub struct Vocabulary {
    // TODO: Option is temp for back compatibility
    eos_token_id: Option<TokenId>,
    map: HashMap<Token, Vec<TokenId>>,
}

impl Vocabulary {
    /// Creates an empty vocabulary.
    pub fn new(eos_token_id: Option<TokenId>) -> Self {
        Self {
            eos_token_id,
            map: HashMap::new(),
        }
    }

    /// Creates the vocabulary of pre-trained model from Hugging Face Hub.
    pub fn from_pretrained(
        model: &str,
        parameters: Option<FromPretrainedParameters>,
    ) -> Result<Self, VocabularyError> {
        let mut tokenizer =
            Tokenizer::from_pretrained(model, parameters.clone()).map_err(|error| {
                VocabularyError::UnableToCreateTokenizer {
                    model: model.to_string(),
                    source: TokenizerError(error),
                }
            })?;
        Self::filter_prepend_normalizers(&mut tokenizer);

        let eos_token_id = EosTokenLocator::locate(model, &tokenizer, &parameters);
        let Some(eos_token_id) = eos_token_id else {
            return Err(VocabularyError::UnableToLocateEosTokenId {
                model: model.to_string(),
            });
        };

        Vocabulary::try_from((&mut tokenizer, eos_token_id))
    }

    /// Per provided token returns vector of `TokenId`s if available in the vocabulary.
    pub fn token_to_ids(&self, token: &str) -> Option<&Vec<TokenId>> {
        self.map.get(token)
    }

    /// Gets the identifier of the special end of the sentence token.
    pub fn eos_token_id(&self) -> Option<TokenId> {
        self.eos_token_id
    }

    /// Filters out `Prepend` kind of tokenizer's normalizers.
    fn filter_prepend_normalizers(tokenizer: &mut Tokenizer) {
        // Main concern is prepend normalizers, for example https://github.com/google/sentencepiece
        // In `sentencepiece` tokenizer, `▁` is used to denote spaces in the source text,
        // e.g. `Hello World.` could be tokenized as: [Hello] [▁Wor] [ld] [.]
        //
        // We don't want to deal with the special characters, so we remove `Prepend` normalizers.
        if let Some(normalizer) = tokenizer.get_normalizer() {
            match normalizer {
                NormalizerWrapper::Sequence(normalization_sequence) => {
                    let new_sequence = Sequence::new(
                        normalization_sequence
                            .get_normalizers()
                            .iter()
                            .filter_map(|normalizer| match normalizer {
                                NormalizerWrapper::Prepend(_) => None,
                                _ => Some(normalizer.clone()),
                            })
                            .collect(),
                    );
                    tokenizer.with_normalizer(new_sequence.into());
                }
                NormalizerWrapper::Prepend(_) => {
                    tokenizer.with_normalizer(None::<NormalizerWrapper>);
                }
                _ => {}
            }
        }
    }
}

impl TryFrom<(&mut Tokenizer, u32)> for Vocabulary {
    type Error = VocabularyError;

    fn try_from(value: (&mut Tokenizer, u32)) -> Result<Vocabulary, VocabularyError> {
        let (tokenizer, eos_token_id) = value;

        let mut vocabulary = Vocabulary::new(Some(eos_token_id));
        for (id, added_token) in tokenizer.get_added_tokens_decoder().iter() {
            if !added_token.special {
                vocabulary = vocabulary.insert(added_token.content.clone(), *id);
            }
        }

        let processor = TokenProcessor::new(tokenizer)?;
        for (token, token_id) in tokenizer.get_vocab(false) {
            let token_bytes = processor.process(token)?;
            // TODO: lossy is temp:
            // - in python in was handled by byte_symbol function
            // - interface needs to be redefined to treat Token type as bytes: Vec<u8>
            let processed_token = String::from_utf8_lossy(&token_bytes);
            vocabulary = vocabulary.insert(processed_token, token_id);
        }

        Ok(vocabulary)
    }
}

impl Vocabulary {
    /// Inserts a token to the vocabulary with the specified identifier.
    pub fn insert(mut self, token: impl Into<Token>, id: TokenId) -> Vocabulary {
        self.insert_in_place(token, id);
        self
    }

    /// Extends the vocabulary with tokens and their identifiers.
    pub fn extend<T: Into<Token>, I: IntoIterator<Item = TokenId>>(
        mut self,
        tokens_and_ids: impl IntoIterator<Item = (T, I)>,
    ) -> Vocabulary {
        self.extend_in_place(tokens_and_ids);
        self
    }
}

impl Vocabulary {
    /// Inserts a token to the vocabulary with the specified identifier, in place.
    pub fn insert_in_place(&mut self, token: impl Into<Token>, id: TokenId) {
        // TODO: return error if eos token id is inserted
        let token = token.into();
        self.map.entry(token).or_default().push(id);
    }

    /// Extends the vocabulary with tokens and their identifiers, in place.
    pub fn extend_in_place<T: Into<Token>, I: IntoIterator<Item = TokenId>>(
        &mut self,
        tokens_and_ids: impl IntoIterator<Item = (T, I)>,
    ) {
        for (token, ids) in tokens_and_ids.into_iter() {
            let token = token.into();
            self.map.entry(token).or_default().extend(ids);
        }
    }
}

impl std::ops::Deref for Vocabulary {
    type Target = HashMap<Token, Vec<TokenId>>;

    fn deref(&self) -> &HashMap<Token, Vec<TokenId>> {
        &self.map
    }
}

impl std::fmt::Display for Vocabulary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, (token, token_ids)) in self.iter().enumerate() {
            if index != (self.len() - 1) {
                writeln!(f, "{:?} -> {:?}", token, token_ids)?;
            } else {
                write!(f, "{:?} -> {:?}", token, token_ids)?;
            }
        }
        Ok(())
    }
}

impl From<HashMap<Token, Vec<TokenId>>> for Vocabulary {
    fn from(map: HashMap<Token, Vec<TokenId>>) -> Vocabulary {
        Vocabulary {
            eos_token_id: None,
            map,
        }
    }
}

impl<T, I> FromIterator<(T, I)> for Vocabulary
where
    T: Into<Token>,
    I: IntoIterator<Item = TokenId>,
{
    fn from_iter<A: IntoIterator<Item = (T, I)>>(tokens_and_ids: A) -> Self {
        Vocabulary::new(None).extend(tokens_and_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert() {
        let vocabulary = Vocabulary::new(None)
            .insert("blah", 0)
            .insert("1a", 1)
            .insert("2", 2)
            .insert("0", 3);

        assert_eq!(vocabulary.len(), 4);
        assert_eq!(vocabulary["blah"], &[0]);
        assert_eq!(vocabulary["1a"], &[1]);
        assert_eq!(vocabulary["2"], &[2]);
        assert_eq!(vocabulary["0"], &[3]);
    }

    #[test]
    fn extend() {
        let vocabulary = Vocabulary::new(None).extend([
            ("blah", vec![0]),
            ("1a", vec![1]),
            ("2", vec![2]),
            ("0", vec![3]),
        ]);

        assert_eq!(vocabulary.len(), 4);
        assert_eq!(vocabulary["blah"], &[0]);
        assert_eq!(vocabulary["1a"], &[1]);
        assert_eq!(vocabulary["2"], &[2]);
        assert_eq!(vocabulary["0"], &[3]);
    }

    #[test]
    fn pretrained_from_gpt2() {
        let model = "openai-community/gpt2";
        let tokenizer = Tokenizer::from_pretrained(model, None).expect("Tokenizer failed");
        let vocabulary = Vocabulary::from_pretrained(model, None).expect("Vocabulary failed");

        let v_eos = vocabulary.eos_token_id.expect("No eos token in vocabulary");
        assert_eq!(v_eos, 50256);
        assert_eq!(
            tokenizer.id_to_token(v_eos).expect("Token not found"),
            "<|endoftext|>"
        );

        let token = "Ġal";
        assert!(vocabulary.token_to_ids(token).is_none());
        assert!(tokenizer.token_to_id(token).is_some());

        for (v_token, t_token_expected) in [("abc", "abc"), (" O", "ĠO")] {
            let v_ids = vocabulary.token_to_ids(v_token);
            assert!(v_ids.is_some());
            for v_id in v_ids.unwrap() {
                let t_token = tokenizer
                    .id_to_token(*v_id)
                    .expect("Token id not found in tokenizer");
                assert_eq!(&t_token, t_token_expected);
            }
        }
    }

    #[test]
    fn pretrained_from_llama() {
        let model = "hf-internal-testing/llama-tokenizer";
        let tokenizer = Tokenizer::from_pretrained(model, None).expect("Tokenizer failed");
        let vocabulary = Vocabulary::from_pretrained(model, None).expect("Vocabulary failed");

        let v_eos = vocabulary.eos_token_id.expect("No eos token in vocabulary");
        assert_eq!(v_eos, 2);
        assert_eq!(
            tokenizer.id_to_token(v_eos).expect("Token not found"),
            "</s>"
        );

        for (v_token, t_token_expected) in [
            ("abc", "abc"),
            (" al", "▁al"),
            (" O", "▁O"),
            ("   ", "▁▁▁"),
            // TODO: won't pass since first we need to change token's type to bytes
            // ("<0xFF>", "ÿ"),
            // ("<0x20>", "▁"),
        ] {
            let v_ids = vocabulary.token_to_ids(v_token);
            assert!(v_ids.is_some());
            for v_id in v_ids.unwrap() {
                let t_token = tokenizer
                    .id_to_token(*v_id)
                    .expect("Token id not found in tokenizer");
                assert_eq!(&t_token, t_token_expected);
            }
        }
    }

    #[test]
    fn token_processor_error() {
        let model = "hf-internal-testing/tiny-random-XLMRobertaXLForCausalLM";
        let vocabulary = Vocabulary::from_pretrained(model, None);

        assert!(vocabulary.is_err());
        if let Err(e) = vocabulary {
            assert_eq!(
                e,
                VocabularyError::TokenProcessorError(
                    crate::TokenProcessorError::UnsupportedTokenizer
                )
            )
        }
    }

    #[test]
    fn tokenizer_error() {
        let model = "hf-internal-testing/some-non-existent-model";
        let vocabulary = Vocabulary::from_pretrained(model, None);

        assert!(vocabulary.is_err());
        if let Err(VocabularyError::UnableToCreateTokenizer { model, source }) = vocabulary {
            assert_eq!(model, model.to_string());
            assert_eq!(source.to_string(), "Tokenizer error".to_string());
        }
    }

    #[test]
    fn prepend_normalizers_filtered_out() {
        use tokenizers::normalizers::{Prepend, Sequence};

        let prepend = Prepend::new("_".to_string());
        let prepend_normalizer = NormalizerWrapper::Prepend(prepend);
        let sequence = Sequence::new(vec![prepend_normalizer.clone()]);
        let sequence_normalizer = NormalizerWrapper::Sequence(sequence);

        let model = "hf-internal-testing/llama-tokenizer";
        let tokenizer = Tokenizer::from_pretrained(model, None).expect("Tokenizer failed");

        for normalizer in [prepend_normalizer, sequence_normalizer] {
            let mut normalized_t = tokenizer.clone();
            normalized_t.with_normalizer(Some(normalizer));
            Vocabulary::filter_prepend_normalizers(&mut normalized_t);
            if let Some(n) = normalized_t.get_normalizer() {
                match n {
                    NormalizerWrapper::Sequence(seq) => {
                        for n in seq.get_normalizers() {
                            if let NormalizerWrapper::Prepend(_) = n {
                                unreachable!()
                            }
                        }
                    }
                    NormalizerWrapper::Prepend(_) => unreachable!(),
                    _ => {}
                }
            }
        }
    }
}
