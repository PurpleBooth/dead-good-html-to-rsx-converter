use miette::Diagnostic;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug, Diagnostic)]
pub enum Error {
    #[error("failed to parse html")]
    HtmlParseError(#[from] tl::errors::ParseError),
}
