#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("not authenticated — run 'jot serve' first")]
    NotAuthenticated,
    #[error("server error: {0}")]
    Server(String),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("config error: {0}")]
    Config(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
