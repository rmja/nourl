#![no_std]
#[cfg(feature = "defmt")]
mod defmt_impl;
mod error;

use core::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

pub use error::Error;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A parsed URL to extract different parts of the URL.
pub struct Url<'a> {
    scheme: UrlScheme,
    host: &'a str,
    is_host_ipv6: bool,
    port: Option<u16>,
    path: &'a str,
}

impl core::fmt::Debug for Url<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}://", self.scheme.as_str())?;
        if self.is_host_ipv6 {
            write!(f, "[{}]", self.host)?;
        } else {
            write!(f, "{}", self.host)?;
        }
        if let Some(port) = self.port {
            write!(f, ":{}", port)?
        }
        write!(f, "{}", self.path)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Url<'_> {
    fn format(&self, f: defmt::Formatter) {
        use defmt::write;
        write!(f, "{}://", self.scheme.as_str());
        if self.is_host_ipv6 {
            write!(f, "[{}]", self.host);
        } else {
            write!(f, "{}", self.host);
        }
        if let Some(port) = self.port {
            write!(f, ":{}", port)
        }
        write!(f, "{}", self.path)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum UrlScheme {
    /// HTTP scheme
    HTTP,
    /// HTTPS (HTTP + TLS) scheme
    HTTPS,
    /// MQTT scheme
    MQTT,
    /// MQTTS (MQTT + TLS) scheme
    MQTTS,
}

impl UrlScheme {
    /// str representation of the scheme
    ///
    /// The returned str is always lowercase
    pub fn as_str(&self) -> &str {
        match self {
            UrlScheme::HTTP => "http",
            UrlScheme::HTTPS => "https",
            UrlScheme::MQTT => "mqtt",
            UrlScheme::MQTTS => "mqtts",
        }
    }

    /// Get the default port for scheme
    pub const fn default_port(&self) -> u16 {
        match self {
            UrlScheme::HTTP => 80,
            UrlScheme::HTTPS => 443,
            UrlScheme::MQTT => 1883,
            UrlScheme::MQTTS => 8883,
        }
    }
}

impl<'a> Url<'a> {
    /// Parse the provided url
    pub fn parse(url: &'a str) -> Result<Url<'a>, Error> {
        // Split out the scheme.
        let mut parts = url.split("://");
        // This can't fail, since `Split` always yields `Some` on the first iteration.
        let scheme = parts.next().unwrap();
        let host_port_path = parts.next().ok_or(Error::NoScheme)?;

        let scheme = if scheme.eq_ignore_ascii_case("http") {
            Ok(UrlScheme::HTTP)
        } else if scheme.eq_ignore_ascii_case("https") {
            Ok(UrlScheme::HTTPS)
        } else {
            Err(Error::UnsupportedScheme)
        }?;

        // Split host and path first
        let (host_port, path) = if let Some(path_delim) = host_port_path.find('/') {
            let host_port = &host_port_path[..path_delim];
            let path = &host_port_path[path_delim..];
            let path = if path.is_empty() { "/" } else { path };
            (host_port, path)
        } else {
            (host_port_path, "/")
        };

        // Now handle the port
        let (host, port, is_host_ipv6) = if host_port.starts_with('[') {
            // If we are here, a '[' was found, indicating that the host is an IPv6 address. If
            // there is no closing ']' we return Ipv6AddressInvalid here.
            let ipv6_addr_end = host_port.find(']').ok_or(Error::Ipv6AddressInvalid)?;
            // Check if there's a port following the IPv6 address.
            let port = if let Some(port) = host_port
                .get(ipv6_addr_end + 1..)
                .filter(|port| !port.is_empty())
            {
                Some(
                    port.strip_prefix(':')
                        .ok_or(Error::LeftoverTokensAfterIpv6)?,
                )
            } else {
                None
            };
            (&host_port[1..ipv6_addr_end], port, true)
        } else if let Some(port_delim) = host_port.find(':') {
            // The hostname is followed by a port, which we attempt to extract here.
            (
                &host_port[..port_delim],
                host_port.get(port_delim + 1..),
                false,
            )
        } else {
            // No port follows the hostname.
            (host_port, None, false)
        };
        if port == Some("") {
            return Err(Error::NoPortAfterColon);
        }
        let port = port
            .map(|port| port.parse::<u16>())
            .transpose()
            .map_err(|_| Error::InvalidPort)?;

        Ok(Self {
            scheme,
            host,
            is_host_ipv6,
            path,
            port,
        })
    }

    /// Get the url scheme
    pub fn scheme(&self) -> UrlScheme {
        self.scheme
    }

    /// Get the url host
    pub fn host(&self) -> &'a str {
        self.host
    }

    /// Attempt to get the url host as an IP address
    ///
    /// This will only work, if the url host was actually specified as an IP address.
    pub fn host_ip(&self) -> Option<IpAddr> {
        if self.is_host_ipv6 {
            Ipv6Addr::from_str(self.host).ok().map(|ip| ip.into())
        } else {
            Ipv4Addr::from_str(self.host).ok().map(|ip| ip.into())
        }
    }

    /// Get the url port if specified
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Get the url port or the default port for the scheme
    pub fn port_or_default(&self) -> u16 {
        self.port.unwrap_or_else(|| self.scheme.default_port())
    }

    /// Get the url path
    pub fn path(&self) -> &'a str {
        self.path
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;

    #[test]
    fn test_parse_no_scheme() {
        assert_eq!(Error::NoScheme, Url::parse("").err().unwrap());
        assert_eq!(Error::NoScheme, Url::parse("http:/").err().unwrap());
    }

    #[test]
    fn test_parse_unsupported_scheme() {
        assert_eq!(
            Error::UnsupportedScheme,
            Url::parse("something://").err().unwrap()
        );
    }

    #[test]
    fn test_parse_no_host() {
        let url = Url::parse("http://").unwrap();
        assert_eq!(url.scheme(), UrlScheme::HTTP);
        assert_eq!(url.host(), "");
        assert_eq!(url.port_or_default(), 80);
        assert_eq!(url.path(), "/");
    }

    #[test]
    fn test_parse_minimal() {
        let url = Url::parse("http://localhost").unwrap();
        assert_eq!(url.scheme(), UrlScheme::HTTP);
        assert_eq!(url.host(), "localhost");
        assert_eq!(url.port_or_default(), 80);
        assert_eq!(url.path(), "/");

        assert_eq!("http://localhost/", std::format!("{:?}", url));
    }

    #[test]
    fn test_parse_path() {
        let url = Url::parse("http://localhost/foo/bar").unwrap();
        assert_eq!(url.scheme(), UrlScheme::HTTP);
        assert_eq!(url.host(), "localhost");
        assert_eq!(url.port_or_default(), 80);
        assert_eq!(url.path(), "/foo/bar");

        assert_eq!("http://localhost/foo/bar", std::format!("{:?}", url));
    }

    #[test]
    fn test_parse_path_with_colon() {
        let url = Url::parse("http://localhost/foo/bar:123").unwrap();
        assert_eq!(url.scheme(), UrlScheme::HTTP);
        assert_eq!(url.host(), "localhost");
        assert_eq!(url.port_or_default(), 80);
        assert_eq!(url.path(), "/foo/bar:123");

        assert_eq!("http://localhost/foo/bar:123", std::format!("{:?}", url));
    }

    #[test]
    fn test_parse_port() {
        let url = Url::parse("http://localhost:8088").unwrap();
        assert_eq!(url.scheme(), UrlScheme::HTTP);
        assert_eq!(url.host(), "localhost");
        assert_eq!(url.port().unwrap(), 8088);
        assert_eq!(url.path(), "/");

        assert_eq!("http://localhost:8088/", std::format!("{:?}", url));
    }

    #[test]
    fn test_parse_port_path() {
        let url = Url::parse("http://localhost:8088/foo/bar").unwrap();
        assert_eq!(url.scheme(), UrlScheme::HTTP);
        assert_eq!(url.host(), "localhost");
        assert_eq!(url.port().unwrap(), 8088);
        assert_eq!(url.path(), "/foo/bar");

        assert_eq!("http://localhost:8088/foo/bar", std::format!("{:?}", url));
    }

    #[test]
    fn test_parse_scheme() {
        let url = Url::parse("https://localhost/").unwrap();
        assert_eq!(url.scheme(), UrlScheme::HTTPS);
        assert_eq!(url.host(), "localhost");
        assert_eq!(url.port_or_default(), 443);
        assert_eq!(url.path(), "/");

        assert_eq!("https://localhost/", std::format!("{:?}", url));
    }
    #[test]
    fn test_parse_ipv4() {
        let url = Url::parse("https://127.0.0.1:1337/foo/bar").unwrap();
        assert_eq!(url.scheme(), UrlScheme::HTTPS);
        assert_eq!(url.host(), "127.0.0.1");
        assert_eq!(
            url.host_ip().unwrap(),
            IpAddr::from_str("127.0.0.1").unwrap()
        );
        assert_eq!(url.port_or_default(), 1337);
        assert_eq!(url.path(), "/foo/bar");

        assert_eq!("https://127.0.0.1:1337/foo/bar", std::format!("{:?}", url));
    }
    #[test]
    fn test_parse_ipv6() {
        let url = Url::parse("https://[fe80::]/foo/bar").unwrap();
        assert_eq!(url.scheme(), UrlScheme::HTTPS);
        assert_eq!(url.host(), "fe80::");
        assert_eq!(url.host_ip().unwrap(), IpAddr::from_str("fe80::").unwrap());
        assert_eq!(url.port_or_default(), 443);
        assert_eq!(url.path(), "/foo/bar");

        assert_eq!("https://[fe80::]/foo/bar", std::format!("{:?}", url));
    }
    #[test]
    fn test_parse_ipv6_port() {
        let url = Url::parse("https://[fe80::]:1337/foo/bar").unwrap();
        assert_eq!(url.scheme(), UrlScheme::HTTPS);
        assert_eq!(url.host(), "fe80::");
        assert_eq!(url.host_ip().unwrap(), IpAddr::from_str("fe80::").unwrap());
        assert_eq!(url.port_or_default(), 1337);
        assert_eq!(url.path(), "/foo/bar");

        assert_eq!("https://[fe80::]:1337/foo/bar", std::format!("{:?}", url));
    }
    #[test]
    fn test_invalid_ipv6() {
        assert_eq!(
            Url::parse("http://[fe80::/"),
            Err(Error::Ipv6AddressInvalid)
        );
    }
    #[test]
    fn test_leftover_tokens_ipv6() {
        assert_eq!(
            Url::parse("http://[fe80]a/"),
            Err(Error::LeftoverTokensAfterIpv6)
        );
    }
    #[test]
    fn test_no_port_after_colon() {
        assert_eq!(
            Url::parse("http://localhost:/"),
            Err(Error::NoPortAfterColon)
        );
        assert_eq!(
            Url::parse("http://[fe80::]:/"),
            Err(Error::NoPortAfterColon)
        );
    }
    #[test]
    fn test_invalid_port() {
        assert_eq!(
            Url::parse("http://localhost:12E4/"),
            Err(Error::InvalidPort)
        );
        assert_eq!(Url::parse("http://[fe80::]:12E4/"), Err(Error::InvalidPort));
    }
}
