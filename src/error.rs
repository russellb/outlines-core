use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("The vocabulary does not allow us to build a sequence that matches the input")]
    IndexError,
    #[error("Unable to create tokenizer for {model}")]
    UnableToCreateTokenizer { model: String },
    #[error("Unable to locate EOS token for {model}")]
    UnableToLocateEosTokenId { model: String },
    #[error("Tokenizer is not supported by token processor")]
    UnsupportedByTokenProcessor,
    #[error("Decoder unpacking failed for token processor")]
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
