pub mod ssl;

use serde::Deserialize;
use dotenv::dotenv;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub port:           u16,
    pub sport:          u16,
    pub host:           String,
    pub pk:             String,
    pub cc:             String,
    pub key:            String,
    pub appname:        String,
    pub description:    String,
    pub sessid:         String,
 }

impl Config {
    pub fn init() -> Result<Config, config::ConfigError> {
        dotenv().ok();

        let mut c = config::Config::new();
        c.merge(config::Environment::default())?;

        c.try_into()
    }
}
