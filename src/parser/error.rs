use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("{file}:{line}: {message}")]
    Syntax {
        file: PathBuf,
        line: usize,
        message: String,
    },

    #[error("{file}: {source}")]
    Io {
        file: PathBuf,
        source: std::io::Error,
    },
}

impl ParseError {
    pub fn syntax(file: impl Into<PathBuf>, line: usize, message: impl Into<String>) -> Self {
        ParseError::Syntax {
            file: file.into(),
            line,
            message: message.into(),
        }
    }

    pub fn io(file: impl Into<PathBuf>, source: std::io::Error) -> Self {
        ParseError::Io {
            file: file.into(),
            source,
        }
    }
}
