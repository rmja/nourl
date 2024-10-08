#![no_std]
#[cfg(feature = "defmt")]
mod defmt_impl;
mod error;

pub use error::Error;

/// A parsed URL to extract different parts of the URL.
pub struct Url<'a> {
    scheme: UrlScheme,
    host: &'a str,
    port: Option<u16>,
    path: &'a str,
}

impl core::fmt::Debug for Url<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(port) = self.port {
            write!(
                f,
                "{}://{}:{}{}",
                self.scheme.as_str(),
                self.host,
                port,
                self.path
            )
        } else {
            write!(f, "{}://{}{}", self.scheme.as_str(), self.host, self.path)
        }
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Url<'_> {
    fn format(&self, fmt: defmt::Formatter) {
        if let Some(port) = self.port {
            defmt::write!(
                fmt,
                "{}://{}:{}{}",
                self.scheme.as_str(),
                self.host,
                port,
                self.path
            )
        } else {
            defmt::write!(fmt, "{}://{}{}", self.scheme.as_str(), self.host, self.path)
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
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
    #[cfg(not(feature = "url-lite-backend"))]
    pub fn parse(url: &'a str) -> Result<Url<'a>, Error> {
        let mut parts = url.split("://");
        let scheme = parts.next().unwrap();
        let host_port_path = parts.next().ok_or(Error::NoScheme)?;

        let scheme = if scheme.eq_ignore_ascii_case("http") {
            Ok(UrlScheme::HTTP)
        } else if scheme.eq_ignore_ascii_case("https") {
            Ok(UrlScheme::HTTPS)
        } else {
            Err(Error::UnsupportedScheme)
        }?;

        let (host, port, path) = if let Some(port_delim) = host_port_path.find(':') {
            // Port is defined
            let host = &host_port_path[..port_delim];
            let rest = &host_port_path[port_delim..];

            let (port, path) = if let Some(path_delim) = rest.find('/') {
                let port = rest[1..path_delim].parse::<u16>().ok();
                let path = &rest[path_delim..];
                let path = if path.is_empty() { "/" } else { path };
                (port, path)
            } else {
                let port = rest[1..].parse::<u16>().ok();
                (port, "/")
            };
            (host, port, path)
        } else {
            let (host, path) = if let Some(needle) = host_port_path.find('/') {
                let host = &host_port_path[..needle];
                let path = &host_port_path[needle..];
                (host, if path.is_empty() { "/" } else { path })
            } else {
                (host_port_path, "/")
            };
            (host, None, path)
        };

        Ok(Self {
            scheme,
            host,
            path,
            port,
        })
    }

    /// Parse the provided url
    #[cfg(feature = "url-lite-backend")]
    pub fn parse(url: &'a str) -> Result<Url<'a>, Error> {
        let url = url_lite::Url::parse(url).map_err(|err| match err {
            url_lite::ParseError::EmptyInput => Error::NoScheme,
            url_lite::ParseError::InvalidConnect
            | url_lite::ParseError::Whitespace
            | url_lite::ParseError::NoHost
            | url_lite::ParseError::Invalid => Error::InvalidUrl,
        })?;

        let scheme_str = url.schema.ok_or(Error::NoScheme)?;

        let scheme = if scheme_str.eq_ignore_ascii_case("http") {
            UrlScheme::HTTP
        } else if scheme_str.eq_ignore_ascii_case("https") {
            UrlScheme::HTTPS
        } else if scheme_str.eq_ignore_ascii_case("mqtt") {
            UrlScheme::MQTT
        } else if scheme_str.eq_ignore_ascii_case("mqtts") {
            UrlScheme::MQTTS
        } else {
            return Err(Error::InvalidUrl);
        };

        let port = if let Some(port) = url.port {
            Some(port.parse::<u16>().map_err(|_| Error::InvalidUrl)?)
        } else {
            None
        };

        Ok(Self {
            scheme,
            host: url.host.unwrap_or_default(),
            port,
            path: url.path.unwrap_or("/"),
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
    #[cfg(not(feature = "url-lite-backend"))]
    fn test_parse_no_scheme() {
        assert_eq!(Error::NoScheme, Url::parse("").err().unwrap());
        assert_eq!(Error::NoScheme, Url::parse("http:/").err().unwrap());
    }

    #[test]
    #[cfg(not(feature = "url-lite-backend"))]
    fn test_parse_unsupported_scheme() {
        assert_eq!(
            Error::UnsupportedScheme,
            Url::parse("something://").err().unwrap()
        );
    }

    #[test]
    #[cfg(not(feature = "url-lite-backend"))]
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
}
