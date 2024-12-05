// These is a test file for using the Awesense REST API to get data from the Awesense platform.
// Using the Awesense API requires both an API key and encrypted user credentials.

use axum::http::response;
use tokio;
use reqwest::{Client,Response};
use serde::{Deserialize, Serialize};
use tracing::Level;

use bcit_smart::{AwesenseApiInfo, AwesenseGrid};


const API_KEY_HEADER_NAME: &str = "Ocp-Apim-Subscription-Key";
const ENCRYPTED_USER_CREDENTIALS_HEADER_NAME: &str = "Authorization";
const ENDPOINT: &str = "https://api-connect.awesense.com/basic_data_retrieval/api/v1/grid";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bcit_smart::create_customized_tracing_subscriber(Some(Level::DEBUG));

    // Load the API key and encrypted user credentials from the config file
    let config: AwesenseApiInfo = bcit_smart::load_config("awesense_config.ron")?;
    println!("config: {:#?}", config);

    // Create a new reqwest client
    let client = Client::new();

    // Create a new request builder
    let response = client.get(ENDPOINT)
        .header(API_KEY_HEADER_NAME, &config.api_key)
        .header(ENCRYPTED_USER_CREDENTIALS_HEADER_NAME, &config.encrypted_user_credentials)
        .send().await?;
    println!("Response status code: {}", response.status());
    
    // Only parse the response body as JSON if the status code is 200
    if response.status() != 200 {
        println!("Response: {:#?}", response.text().await?);
        return Ok(());
    }

    // Parse the response body as JSON
    let body = response.json::<Vec<AwesenseGrid>>().await?;

    // Print the response body
    println!("Response body: {:#?}", body);

    Ok(())
}