use reqwest::RequestBuilder;

#[derive(Clone)]
pub enum Auth {
    Basic { email: String, token: String },
    Bearer { access_token: String, email: String },
}

impl Auth {
    pub fn basic(email: &str, token: &str) -> Self {
        Self::Basic {
            email: email.to_string(),
            token: token.to_string(),
        }
    }

    pub fn bearer(access_token: impl Into<String>, email: impl Into<String>) -> Self {
        Self::Bearer {
            access_token: access_token.into(),
            email: email.into(),
        }
    }

    pub fn email(&self) -> &str {
        match self {
            Self::Basic { email, .. } | Self::Bearer { email, .. } => email,
        }
    }

    pub fn apply(&self, req: RequestBuilder) -> RequestBuilder {
        match self {
            Self::Basic { email, token } => req.basic_auth(email, Some(token.as_str())),
            Self::Bearer { access_token, .. } => req.bearer_auth(access_token),
        }
    }
}
