mod error;
mod model;
mod routes;
mod services;

use std::env;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};

use dotenv::dotenv;
use reqwest::Client;
use tower_http::cors::{Any, CorsLayer};

use crate::routes::api;

#[derive(Clone)]
pub struct AppState {
    client: Client,
    config: AppConfig,
}

impl AppState {
    pub fn new(client: Client, config: AppConfig) -> Self {
        Self { client, config }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn api_key(&self) -> &str {
        &self.config.api_key
    }

    pub fn current_market(&self) -> &str {
        &self.config.current_market
    }

    pub fn wallet_allocations_path(&self) -> &Path {
        &self.config.wallet_allocations_path
    }

    pub fn barca_allocations_path(&self) -> &Path {
        &self.config.barca_allocations_path
    }

    pub fn bind_address(&self) -> IpAddr {
        self.config.bind_address
    }

    pub fn port(&self) -> u16 {
        self.config.port
    }
}

#[derive(Clone)]
pub struct AppConfig {
    api_key: String,
    current_market: String,
    wallet_allocations_path: PathBuf,
    barca_allocations_path: PathBuf,
    bind_address: IpAddr,
    port: u16,
}

impl AppConfig {
    fn from_env() -> Result<Self, ConfigError> {
        let api_key = env::var("API_KEY").map_err(|_| ConfigError::MissingEnv("API_KEY"))?;
        let current_market =
            env::var("CURRENT_MARKET").unwrap_or_else(|_| "BullMarket".to_string());
        let wallet_allocations_path = env::var("WALLET_ALLOCATIONS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("wallet_allocations.csv"));
        let barca_allocations_path = env::var("BARCA_ALLOCATIONS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("wallet_barca.csv"));

        let bind_address = env::var("APP_HOST")
            .or_else(|_| env::var("APP_ADDRESS"))
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        let bind_address = bind_address
            .parse::<IpAddr>()
            .map_err(|_| ConfigError::InvalidAddress)?;

        let port = match env::var("APP_PORT") {
            Ok(value) => value.parse::<u16>().map_err(|_| ConfigError::InvalidPort)?,
            Err(_) => 3001,
        };

        Ok(Self {
            api_key,
            current_market,
            wallet_allocations_path,
            barca_allocations_path,
            bind_address,
            port,
        })
    }

    pub fn bind_address(&self) -> IpAddr {
        self.bind_address
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

#[derive(Debug)]
enum ConfigError {
    MissingEnv(&'static str),
    InvalidAddress,
    InvalidPort,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingEnv(name) => {
                write!(f, "Missing required environment variable: {}", name)
            }
            ConfigError::InvalidAddress => {
                write!(f, "APP_HOST/APP_ADDRESS must be a valid IP address")
            }
            ConfigError::InvalidPort => write!(f, "APP_PORT must be a valid u16 value"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let config = AppConfig::from_env().map_err(|err| {
        eprintln!("Configuration error: {}", err);
        let err: Box<dyn std::error::Error> = Box::new(err);
        err
    })?;

    let port = config.port();
    let bind_address = config.bind_address();
    let client = Client::builder().no_proxy().build().map_err(|err| {
        eprintln!("Failed to build HTTP client: {}", err);
        let err: Box<dyn std::error::Error> = Box::new(err);
        err
    })?;
    let state = AppState::new(client, config);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = api::router(state).layer(cors);

    let addr = SocketAddr::new(bind_address, port);
    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
