#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// The url did not start with <scheme>://
    NoScheme,
    /// The sceme in the url is not known
    UnsupportedScheme,
}
