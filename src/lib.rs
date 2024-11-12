pub mod error;
pub mod index;
pub mod json_schema;
pub mod prelude;
pub mod primitives;
pub mod regex;
pub mod vocabulary;

use error::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(feature = "python-bindings")]
mod python_bindings;
