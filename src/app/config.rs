/// Centralized environment configuration.
/// All env vars and defaults are defined here.
#[derive(Debug, Clone)]
pub struct Config {
    /// Database connection URL. Required.
    pub database_url: String,

    /// Base URL for generating links in emails.
    /// Default: http://localhost:3000
    pub app_url: String,

    /// From/reply address for outgoing emails.
    /// Default: please-configure@example.com
    pub mail_from: String,

    /// Mail adapter: "console" or "smtp".
    /// Default: console
    pub mail_adapter: String,

    /// SMTP host. Required when mail_adapter=smtp.
    pub smtp_host: Option<String>,

    /// SMTP port.
    /// Default: 587
    pub smtp_port: u16,

    /// SMTP username. Optional for some servers.
    pub smtp_user: Option<String>,

    /// SMTP password. Optional for some servers.
    pub smtp_pass: Option<String>,
}

impl Config {
    /// Build config from environment variables.
    /// Returns an error if required vars are missing.
    pub fn from_env() -> Result<Self, String> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL must be set in .env")?;

        let app_url = std::env::var("APP_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());

        let mail_from = std::env::var("MAIL_FROM")
            .unwrap_or_else(|_| "please-configure@example.com".to_string());

        let mail_adapter = std::env::var("MAIL_ADAPTER")
            .unwrap_or_else(|_| "console".to_string());

        let smtp_host = std::env::var("SMTP_HOST").ok();
        let smtp_port = std::env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse::<u16>()
            .map_err(|_| "SMTP_PORT must be a valid port number")?;
        let smtp_user = std::env::var("SMTP_USER").ok();
        let smtp_pass = std::env::var("SMTP_PASS").ok();

        Ok(Self {
            database_url,
            app_url,
            mail_from,
            mail_adapter,
            smtp_host,
            smtp_port,
            smtp_user,
            smtp_pass,
        })
    }

    /// Returns the base URL without trailing slash, for building links.
    pub fn app_url_base(&self) -> &str {
        self.app_url.trim_end_matches('/')
    }

    /// Config for tests. Uses in-memory database URL and console mailer.
    pub fn for_tests() -> Self {
        Self {
            database_url: "sqlite::memory:".to_string(),
            app_url: "http://localhost:3000".to_string(),
            mail_from: "test@example.com".to_string(),
            mail_adapter: "console".to_string(),
            smtp_host: None,
            smtp_port: 587,
            smtp_user: None,
            smtp_pass: None,
        }
    }
}
