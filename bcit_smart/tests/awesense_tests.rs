// Test the basic Awesense API endpoints using Reqwest.

use reqwest::Client;
use tokio;

use bcit_smart::{AwesenseApiInfo, AwesenseGrid};
use odin_actor::prelude::*;

const API_KEY_HEADER_NAME: &str = "Ocp-Apim-Subscription-Key";
const ENCRYPTED_USER_CREDENTIALS_HEADER_NAME: &str = "Authorization";

#[tokio::test]
async fn test_grids_available() -> Result<(), Box<dyn std::error::Error>> {
    bcit_smart::create_customized_tracing_subscriber(None);

    let grid_available_endpoint = "basic_data_retrieval/api/v1/grid";

    let config: AwesenseApiInfo = bcit_smart::load_config("awesense_config.ron")?;
    // Check that all required fields are present in the config
    println!("Loaded config: {:#?}", config);
    assert!(
        !config.api_key.is_empty(),
        "API key is missing from the config file"
    );
    assert!(
        !config.encrypted_user_credentials.is_empty(),
        "Encrypted user credentials are missing from the config file"
    );
    assert!(
        !config.base_url.is_empty(),
        "Base URL is missing from the config file"
    );

    let client = Client::new();
    let response = client
        .get(&format!("{}/{}", config.base_url, grid_available_endpoint))
        .header(API_KEY_HEADER_NAME, &config.api_key)
        .header(ENCRYPTED_USER_CREDENTIALS_HEADER_NAME, &config.encrypted_user_credentials)
        .send().await?;

    // Check that the response status code is 200
    println!("Response status code: {}", response.status());
    assert_eq!(response.status(), 200);

    let body: Vec<AwesenseGrid> = response.json().await?;

    // Check that there are some grids in the response
    println!("Response body: {:#?}", body);
    assert!(
        !body.is_empty(),
        "Expected at least one grid in the response, but none were found"
    );

    Ok(())
}
