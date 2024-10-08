#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// The url did not start with <scheme>://
    NoScheme,
    /// The sceme in the url is not known
    UnsupportedScheme,
    /// The url is invalid
    InvalidUrl,
}
