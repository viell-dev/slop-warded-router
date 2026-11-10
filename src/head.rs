use http::{HeaderMap, Method, Request, Uri, Version, request::Parts};

/// Security of the connection as established by the server or trusted middleware.
///
/// Insert this type into request extensions. Forwarding headers and URI schemes
/// are not automatically trusted. `Unknown` is distinct from known cleartext.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TransportSecurity {
    /// No trustworthy security information was provided.
    #[default]
    Unknown,
    /// The configured trusted context identifies this request as HTTPS.
    Https,
    /// The configured trusted context identifies this request as non-HTTPS.
    Unsecure,
}

/// An immutable request-head view with no body or arbitrary extension access.
#[derive(Clone, Copy, Debug)]
pub struct Head<'a> {
    method: &'a Method,
    uri: &'a Uri,
    version: Version,
    headers: &'a HeaderMap,
    security: TransportSecurity,
}

impl<'a> Head<'a> {
    /// Borrow the head of a request without inspecting or constraining its body.
    pub fn from_request<B>(request: &'a Request<B>) -> Self {
        Self {
            method: request.method(),
            uri: request.uri(),
            version: request.version(),
            headers: request.headers(),
            security: request.extensions().get().copied().unwrap_or_default(),
        }
    }

    /// Borrow detached HTTP request parts.
    pub fn from_parts(parts: &'a Parts) -> Self {
        Self {
            method: &parts.method,
            uri: &parts.uri,
            version: parts.version,
            headers: &parts.headers,
            security: parts.extensions.get().copied().unwrap_or_default(),
        }
    }

    /// The exact request method.
    pub fn method(&self) -> &Method {
        self.method
    }
    /// The URI supplied by the HTTP stack, without router normalization.
    pub fn uri(&self) -> &Uri {
        self.uri
    }
    /// The HTTP version observed by this server, which may be behind a proxy.
    pub fn version(&self) -> Version {
        self.version
    }
    /// The raw header map, including repeated and non-UTF-8 values.
    pub fn headers(&self) -> &HeaderMap {
        self.headers
    }
    /// Trusted HTTPS context; never inferred from a client-provided header.
    pub fn security(&self) -> TransportSecurity {
        self.security
    }
}
