pub mod index;
pub mod json_schema;
pub mod prelude;
pub mod primitives;
pub mod regex;
pub mod vocabulary;

mod locator;
mod processor;

#[cfg(feature = "python-bindings")]
mod python_bindings;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("The vocabulary does not allow us to build a sequence that matches the input")]
    IndexError,
}

#[derive(Error, Debug)]
pub enum VocabularyError {
    #[error("Unable to create tokenizer for {model}, source {source}")]
    UnableToCreateTokenizer {
        model: String,
        source: tokenizers::Error,
    },
    #[error("Unable to locate EOS token for {model}")]
    UnableToLocateEosTokenId { model: String },
    #[error("Unable to process token")]
    TokenProcessorError(#[from] TokenProcessorError),
}

#[derive(Error, Debug)]
pub enum TokenProcessorError {
    #[error("Tokenizer is not supported")]
    UnsupportedTokenizer,
    #[error("Decoder unpacking failed")]
    DecoderUnpackingFailed,
    #[error("Token processing failed for byte level processor")]
    ByteProcessorFailed,
    #[error("Token processing failed for byte fallback level processor")]
    ByteFallbackProcessorFailed,
}

#[cfg(feature = "python-bindings")]
impl From<Error> for pyo3::PyErr {
    fn from(e: Error) -> Self {
        use pyo3::{exceptions::PyValueError, PyErr};
        PyErr::new::<PyValueError, _>(e.to_string())
    }
}
