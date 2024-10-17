//! The native `net` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.14.0", features = ["net"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(rune_modules::net::module(true)?)?;
//! # Ok::<_, rune::support::Error>(())
//! ```
//!
//! Use it in Rune:
//!
//! ```rust,ignore
//! ```

use std::net;

use rune::{Any, ContextError, Module};

/// Construct the `net` module.
#[rune::module(::std::net)]
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;

    module.ty::<SocketAddr>()?;
    module.ty::<IpAddr>()?;

    module.function_meta(SocketAddr::new__meta)?;
    module.function_meta(SocketAddr::ip__meta)?;
    module.function_meta(SocketAddr::set_ip__meta)?;
    module.function_meta(SocketAddr::port__meta)?;
    module.function_meta(SocketAddr::set_port__meta)?;
    module.function_meta(SocketAddr::is_ipv4__meta)?;
    module.function_meta(SocketAddr::is_ipv6__meta)?;

    module.function_meta(IpAddr::is_unspecified__meta)?;
    module.function_meta(IpAddr::is_loopback__meta)?;
    module.function_meta(IpAddr::is_multicast__meta)?;
    module.function_meta(IpAddr::is_ipv4__meta)?;
    module.function_meta(IpAddr::is_ipv6__meta)?;
    module.function_meta(IpAddr::to_canonical__meta)?;

    Ok(module)
}

/// An internet socket address, either IPv4 or IPv6.
#[derive(Debug, Any)]
#[rune(item = ::net)]
pub struct SocketAddr {
    inner: net::SocketAddr,
}

impl SocketAddr {
    /// Creates a new socket address from an IP address and a port number.
    #[rune::function(keep, path = Self::new)]
    pub const fn new(ip: IpAddr, port: u16) -> Self {
        Self {
            inner: net::SocketAddr::new(ip.inner, port),
        }
    }

    /// Returns the IP address associated with this socket address.
    #[rune::function(instance, keep)]
    pub const fn ip(&self) -> IpAddr {
        IpAddr {
            inner: self.inner.ip(),
        }
    }

    /// Changes the IP address associated with this socket address.
    #[rune::function(instance, keep)]
    pub fn set_ip(&mut self, new_ip: IpAddr) {
        self.inner.set_ip(new_ip.inner);
    }

    /// Returns the port number associated with this socket address.
    #[rune::function(instance, keep)]
    pub const fn port(&self) -> u16 {
        self.inner.port()
    }

    /// Changes the port number associated with this socket address.
    #[rune::function(instance, keep)]
    pub fn set_port(&mut self, new_port: u16) {
        self.inner.set_port(new_port);
    }

    /// Returns [`true`] if the IP address in this `SocketAddr` is an
    /// `IPv4` address, and [`false`] otherwise.
    #[rune::function(instance, keep)]
    pub const fn is_ipv4(&self) -> bool {
        self.inner.is_ipv4()
    }

    /// Returns [`true`] if the IP address in this `SocketAddr` is an
    /// `IPv6` address, and [`false`] otherwise.
    #[rune::function(instance, keep)]
    pub const fn is_ipv6(&self) -> bool {
        self.inner.is_ipv6()
    }
}

impl SocketAddr {
    /// Converts [`SocketAddr`] into a [`std::net::SocketAddr`].
    pub const fn into_std(self) -> net::SocketAddr {
        self.inner
    }

    /// Creates a [`SocketAddr`] from a [`std::net::SocketAddr`].
    pub const fn from_std(addr: net::SocketAddr) -> Self {
        Self { inner: addr }
    }
}

/// An IP address, either IPv4 or IPv6.
#[derive(Debug, Any)]
#[rune(item = ::std::net)]
pub struct IpAddr {
    inner: net::IpAddr,
}

impl IpAddr {
    /// Returns [`true`] for the special 'unspecified' address.
    #[rune::function(instance, keep)]
    pub const fn is_unspecified(&self) -> bool {
        self.inner.is_unspecified()
    }

    /// Returns [`true`] if this is a loopback address.
    #[rune::function(instance, keep)]
    pub const fn is_loopback(&self) -> bool {
        self.inner.is_loopback()
    }

    /// Returns [`true`] if this is a multicast address.
    #[rune::function(instance, keep)]
    pub const fn is_multicast(&self) -> bool {
        self.inner.is_multicast()
    }

    /// Returns [`true`] if this address is an `IPv4` address, and [`false`]
    /// otherwise.
    #[rune::function(instance, keep)]
    pub const fn is_ipv4(&self) -> bool {
        self.inner.is_ipv4()
    }

    /// Returns [`true`] if this address is an `IPv6` address, and [`false`]
    /// otherwise.
    #[rune::function(instance, keep)]
    pub const fn is_ipv6(&self) -> bool {
        self.inner.is_ipv6()
    }

    /// Converts this address to an `IpAddr::V4` if it is an IPv4-mapped IPv6 addresses, otherwise it
    /// returns `self` as-is.
    #[rune::function(instance, keep)]
    pub const fn to_canonical(&self) -> IpAddr {
        Self {
            inner: self.inner.to_canonical(),
        }
    }
}

impl IpAddr {
    /// Converts [`IpAddr`] into a [`std::net::IpAddr`].
    pub const fn into_std(self) -> net::IpAddr {
        self.inner
    }

    /// Creates a [`IpAddr`] from a [`std::net::IpAddr`].
    pub const fn from_std(addr: net::IpAddr) -> Self {
        Self { inner: addr }
    }
}
