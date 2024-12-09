// These is a test file for using the Awesense REST API to get data from the Awesense platform.
// Using the Awesense API requires both an API key and encrypted user credentials.

use tokio;
use reqwest::{Client,Response};
use serde::{Deserialize, Serialize};
use tracing::Level;

use bcit_smart::*;
use odin_actor::prelude::*;
use std::env;


const API_KEY_HEADER_NAME: &str = "Ocp-Apim-Subscription-Key";
const ENCRYPTED_USER_CREDENTIALS_HEADER_NAME: &str = "Authorization";
const ENDPOINT: &str = "https://api-connect.awesense.com/basic_data_retrieval/api/v1/grid";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bcit_smart::create_customized_tracing_subscriber(Some(Level::DEBUG));

    /* Using the Awesense REST API */
    // Load the API key and encrypted user credentials from the config file
    let config: AwesenseApiInfo = bcit_smart::load_config("awesense_config.ron")?;
    println!("config: {:#?}", config);

    // Create a new reqwest client for API requests
    let client = Client::new();

    // Use the Awesense API to get available grids
    let grids = get_available_grids_using_rest(&client, &config).await?;
    println!("Available grids: {:#?}", grids);


    /* Using the Awesense Postgres database */
    // Connect to Awesense Postgres database
    let sql_config: AwesenseSqlInfo = bcit_smart::load_config("awesense_sql_config.ron")?;
    let connection_string = format!("postgres://{}:{}@{}:{}/{}",
    sql_config.user, sql_config.password, sql_config.host, sql_config.port, sql_config.db_name);

    println!("Connection string: {}", connection_string);

    let pool = sqlx::postgres::PgPool::connect(&connection_string).await?;

    let rows: Vec<Grid> = sqlx::query_as("SELECT * FROM grid").fetch_all(&pool).await?;
    println!("Rows: {:#?}", rows);

    // Query the grid_element table
    // If this gets worked on in future, should see about changing the "geometry" column to be parsed a better way
    let rows2: Vec<GridElement> = sqlx::query_as(
        r#"
        SELECT 
            grid_id,
            grid_element_id,
            type as type_,
            customer_type,
            phases,
            is_underground,
            is_producer,
            is_consumer,
            is_switchable,
            switch_is_open,
            terminal1_cn,
            terminal2_cn,
            power_flow_direction,
            upstream_grid_element_id,
            ST_AsText(geometry) AS geometry,
            meta
        FROM grid_element
        WHERE grid_id = $1
        LIMIT 200
        "#
    )
    .bind("awefice") // Bind the grid_id parameter
    .fetch_all(&pool)
    .await?;

    println!("Rows2: {:#?}", rows2);


    Ok(())
}

// Use Awesense API to check that there are some grids in the response
async fn get_available_grids_using_rest(client: &Client, config: &AwesenseApiInfo) -> Result<Vec<AwesenseGrid>> {
    let res = client.get(ENDPOINT)
        .header(API_KEY_HEADER_NAME, &config.api_key)
        .header(ENCRYPTED_USER_CREDENTIALS_HEADER_NAME, &config.encrypted_user_credentials)
        .send().await?;
    
    if res.status() != 200 {
        debug!("Response: {:#?}", res.text().await?);
        return Err(BcitSmartError::MiscError("Failed to get available grids".to_string()));
    }

    let body = res.json::<Vec<AwesenseGrid>>().await?;
    debug!("Response body: {:#?}", body);
    Ok(body)
}

// async fn get