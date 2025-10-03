#![no_std]
#[cfg(feature = "defmt")]
mod defmt_impl;
mod error;

pub use crate::error::Error;

use core::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    str::FromStr,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A parsed URL to extract different parts of the URL.
pub struct Url<'a> {
    scheme: UrlScheme,
    host: &'a str,
    is_host_ipv6: bool,
    scope_id: Option<u32>,
    port: Option<u16>,
    path: &'a str,
    query: Option<&'a str>,
}

impl core::fmt::Debug for Url<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}://", self.scheme.as_str())?;
        if self.is_host_ipv6 {
            write!(f, "[{}", self.host)?;
            if let Some(scope_id) = self.scope_id {
                write!(f, "%{}", scope_id)?;
            }
            write!(f, "]")?;
        } else {
            write!(f, "{}", self.host)?;
        }
        if let Some(port) = self.port {
            write!(f, ":{}", port)?;
        }
        if let Some(query) = self.query {
            write!(f, "{}?{}", self.path, query)
        } else {
            write!(f, "{}", self.path)
        }
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Url<'_> {
    fn format(&self, f: defmt::Formatter) {
        use defmt::write;
        write!(f, "{}://", self.scheme.as_str());
        if self.is_host_ipv6 {
            write!(f, "[{}", self.host);
            if let Some(scope_id) = self.scope_id {
                write!(f, "%{}", scope_id);
            }
            write!(f, "]");
        } else {
            write!(f, "{}", self.host);
        }
        if let Some(port) = self.port {
            write!(f, ":{}", port)
        }
        if let Some(query) = self.query {
            write!(f, "{}?{}", self.path, query)
        } else {
            write!(f, "{}", self.path)
        }
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
    ///
    /// The host may be an IP address. An IPv6 address has to be surrounded by square brackets.
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
        } else if scheme.eq_ignore_ascii_case("mqtt") {
            Ok(UrlScheme::MQTT)
        } else if scheme.eq_ignore_ascii_case("mqtts") {
            Ok(UrlScheme::MQTTS)
        } else {
            Err(Error::UnsupportedScheme)
        }?;

        // Split host and path first
        let (host_port, path, query) = if let Some(path_delim) = host_port_path.find('/') {
            let host_port = &host_port_path[..path_delim];
            let path = &host_port_path[path_delim..];
            let path = if path.is_empty() { "/" } else { path };

            // Split out the query if present
            // The query is everything after the first '?'
            let (path, query) = if let Some(query_delim) = path.find('?') {
                let query = &path[query_delim + 1..];
                let path = &path[..query_delim];
                (path, Some(query))
            } else {
                (path, None)
            };

            (host_port, path, query)
        } else {
            (host_port_path, "/", None)
        };

        // Now handle the host, port and scope ID.
        let (host, port, is_host_ipv6, scope_id) = if host_port.starts_with('[') {
            // If we are here, a '[' was found, indicating that the host is an IPv6 address. If
            // there is no closing ']' we return Ipv6AddressInvalid here.
            let address_block_end = host_port.find(']').ok_or(Error::Ipv6AddressInvalid)?;
            // The range in which the actual address is located.
            let mut address_range = 1..address_block_end;
            // Check if there's a scoped id and parse it if it's present. The address_range will
            // also be altered, to only contain the address.
            let scope_id = if let Some(scope_id_start) = host_port[address_range.clone()].find('%')
            {
                address_range = 1..scope_id_start + 1;
                Some(&host_port[scope_id_start + 2..address_block_end])
            } else {
                None
            };
            // Check if there's a port following the IPv6 address.
            let port = if let Some(port) = host_port
                .get(address_block_end + 1..)
                .filter(|port| !port.is_empty())
            {
                Some(
                    port.strip_prefix(':')
                        .ok_or(Error::LeftoverTokensAfterIpv6)?,
                )
            } else {
                None
            };
            (&host_port[address_range], port, true, scope_id)
        } else if let Some(port_delim) = host_port.find(':') {
            // The hostname is followed by a port, which we attempt to extract here.
            (
                &host_port[..port_delim],
                host_port.get(port_delim + 1..),
                false,
                None,
            )
        } else {
            // No port follows the hostname.
            (host_port, None, false, None)
        };
        if port == Some("") {
            return Err(Error::NoPortAfterColon);
        }
        if scope_id == Some("") {
            return Err(Error::NoScopeIdAfterPercent);
        }
        let port = port
            .map(|port| port.parse::<u16>())
            .transpose()
            .map_err(|_| Error::InvalidPort)?;
        let scope_id = scope_id
            .map(|scope_id| scope_id.parse::<u32>())
            .transpose()
            .map_err(|_| Error::InvalidScopeId)?;

        Ok(Self {
            scheme,
            host,
            scope_id,
            is_host_ipv6,
            path,
            port,
            query,
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

    /// Attempt to get the url host socket address
    ///
    /// This will only work, if the url host was an IP address
    pub fn host_socket_address(&self) -> Option<SocketAddr> {
        Some(match self.host_ip()? {
            IpAddr::V4(address) => {
                SocketAddr::V4(SocketAddrV4::new(address, self.port_or_default()))
            }
            IpAddr::V6(address) => SocketAddr::V6(SocketAddrV6::new(
                address,
                self.port_or_default(),
                0,
                self.scope_id_or_default(),
            )),
        })
    }

    /// Get the url port if specified
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Get the url port or the default port for the scheme
    pub fn port_or_default(&self) -> u16 {
        self.port.unwrap_or_else(|| self.scheme.default_port())
    }

    /// Get the scope ID of the IPv6 address specified in the url
    pub fn scope_id(&self) -> Option<u32> {
        self.scope_id
    }

    /// Get the scope ID of the IPv6 address specified in the url or the default scope ID
    pub fn scope_id_or_default(&self) -> u32 {
        self.scope_id.unwrap_or(0)
    }

    /// Get the url path
    pub fn path(&self) -> &'a str {
        self.path
    }

    /// Get the url query if specified
    pub fn query(&self) -> Option<&'a str> {
        self.query
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
        assert_eq!(url.query(), None);

        let url = Url::parse("mqtt://").unwrap();
        assert_eq!(url.scheme(), UrlScheme::MQTT);
        assert_eq!(url.host(), "");
        assert_eq!(url.port_or_default(), 1883);
        assert_eq!(url.path(), "/");
        assert_eq!(url.query(), None);
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
    fn test_parse_path_query() {
        let url = Url::parse("mqtt://localhost/foo/bar?foo=bar&hello=world").unwrap();
        assert_eq!(url.scheme(), UrlScheme::MQTT);
        assert_eq!(url.host(), "localhost");
        assert_eq!(url.port_or_default(), 1883);
        assert_eq!(url.path(), "/foo/bar");
        assert_eq!(url.query(), Some("foo=bar&hello=world"));

        assert_eq!(
            "mqtt://localhost/foo/bar?foo=bar&hello=world",
            std::format!("{:?}", url)
        );
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
            url.host_socket_address().unwrap(),
            SocketAddr::from_str("127.0.0.1:1337").unwrap()
        );
        assert_eq!(url.port_or_default(), 1337);
        assert_eq!(url.path(), "/foo/bar");

        assert_eq!("https://127.0.0.1:1337/foo/bar", std::format!("{:?}", url));
    }

    #[test]
    fn test_parse_ipv6() {
        let url = Url::parse("https://[fe80::%1]/foo/bar").unwrap();
        assert_eq!(url.scheme(), UrlScheme::HTTPS);
        assert_eq!(url.host(), "fe80::");
        assert_eq!(
            url.host_socket_address().unwrap(),
            SocketAddr::from_str("[fe80::%1]:443").unwrap()
        );
        assert_eq!(url.port_or_default(), 443);
        assert_eq!(url.path(), "/foo/bar");

        assert_eq!("https://[fe80::%1]/foo/bar", std::format!("{:?}", url));
    }

    #[test]
    fn test_parse_ipv6_port() {
        let url = Url::parse("https://[fe80::%1]:1337/foo/bar").unwrap();
        assert_eq!(url.scheme(), UrlScheme::HTTPS);
        assert_eq!(url.host(), "fe80::");
        assert_eq!(
            url.host_socket_address().unwrap(),
            SocketAddr::from_str("[fe80::%1]:1337").unwrap()
        );
        assert_eq!(url.port_or_default(), 1337);
        assert_eq!(url.path(), "/foo/bar");

        assert_eq!("https://[fe80::%1]:1337/foo/bar", std::format!("{:?}", url));
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
