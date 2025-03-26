#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// The url did not start with <scheme>://
    NoScheme,
    /// The sceme in the url is not known
    UnsupportedScheme,
    /// The IPv6 address is invalid.
    Ipv6AddressInvalid,
    /// There were tokens between the closing bracket of an IPv6 address and the next slash, that
    /// weren't a colon.
    LeftoverTokensAfterIpv6,
    /// A colon was present, but no port number following it.
    NoPortAfterColon,
    /// The specified port was either out of range or contained invalid tokens.
    InvalidPort,
}
