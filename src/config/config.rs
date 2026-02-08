use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub environment: EnvironmentConfig,
    pub server: ServerConfig,
    pub datasource: DatasourceConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EnvironmentConfig {
    pub env: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: Option<String>,
    pub port: u16,
    pub base_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatasourceConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
}

pub fn load_configuration() -> Result<AppConfig, config::ConfigError> {
    let builder = config::Config::builder()
        .add_source(config::File::with_name("config").required(true))
        .add_source(config::Environment::with_prefix("CORTO").separator("__"));

    let config = builder.build()?;
    config.try_deserialize::<AppConfig>()
}
