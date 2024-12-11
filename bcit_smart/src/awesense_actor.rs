#![allow(unused_variables)]

use crate::*;

use std::fmt::Debug;
// use serde::{Deserialize, Serialize};
// use chrono::{DateTime, Utc};

// This so be info about the total OASIS dataset
// #[derive(Debug)]
// pub struct AwesenseDataSettings {
//     pub connection_info: AwesenseSqlInfo,
//     pub grid_info: Vec<Grid>,
//     connection_pool: sqlx::Pool<Postgres>,
//     connection_string: String
// }

#[derive(Debug)]
pub struct AwesenseDataSet {
    connection_info: AwesenseSqlInfo,
    grid_info: Vec<Grid>,
    connection_pool: sqlx::Pool<Postgres>,
    connection_string: String,
    grid_elements: Vec<GridElement>,
}

impl AwesenseDataSet {
    pub async fn new( info: AwesenseSqlInfo, _grids: Vec<&str>) -> Result<Self> { 
        let connection_string = format!("postgres://{}:{}@{}:{}/{}",    
            info.user, info.password, info.host, info.port, info.db_name);
        let connection_pool = sqlx::postgres::PgPool::connect(&connection_string).await?;
        let grid_info: Vec<Grid> = sqlx::query_as("SELECT * FROM grid").fetch_all(&connection_pool).await?;

        let grid_elements: Vec<GridElement> = sqlx::query_as(
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
        .fetch_all(&connection_pool)
        .await?;

        Ok(AwesenseDataSet { connection_info: info, grid_info, connection_pool, connection_string, grid_elements })
    }

    pub async fn get_trace_elements(&self, trace: &TraceMessage) -> Vec<GridElement> {
        let grid_element_id = &trace.grid_element_id;
        let trace_name = trace.trace_name.as_str();

        // Define the common column list
        const GRID_ELEMENT_COLUMNS: &str = r#"
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
        "#;

        // Choose the appropriate database function based on trace_name
        let sql_query = match trace_name {
            "Connected" => format!(
                "SELECT {} FROM grid_get_connected($1, $2)",
                GRID_ELEMENT_COLUMNS
            ),
            "Down" => format!(
                "SELECT {} FROM grid_get_downstream($1, $2, true)",
                GRID_ELEMENT_COLUMNS
            ),
            "Nearby" => format!(
                "SELECT {} FROM grid_get_nearby($1, $2, 5, 0)", // Adjust default values if needed
                GRID_ELEMENT_COLUMNS
            ),
            "Same Voltage" => format!(
                "SELECT {} FROM grid_get_same_voltage($1, $2)",
                GRID_ELEMENT_COLUMNS
            ),
            "Source" => format!(
                "SELECT {} FROM grid_get_sources($1, $2, true)", // Adjust default values if needed
                GRID_ELEMENT_COLUMNS
            ),
            _ => {
                debug!("Unsupported trace type: {}", trace_name);
                return vec![];
            }
        };

        
        let grid_elements: Vec<GridElement> = sqlx::query_as(&sql_query)
        .bind("awefice") // Change this for other grids later
        .bind(grid_element_id) // Bind the grid_id parameter
        .fetch_all(&self.connection_pool)
        .await.unwrap();

        grid_elements
    }

    pub fn to_json_pretty (&self)->Result<String> {
        Ok(serde_json::to_string_pretty( &self.grid_elements )?)
    }
    pub fn to_json (&self)->Result<String> {
        Ok(serde_json::to_string( &self.grid_elements )?)
    }
}

/// external message to request action execution with the current AwesenseDataSet
#[derive(Debug)] pub struct ExecSnapshotActionAwesense(pub DynDataRefAction<Vec<GridElement>>);
#[derive(Debug)] pub struct TraceDetailsAwesense(pub DynDataRefAction<Vec<GridElement>>, pub TraceMessage);

// internal messages sent by the Actor responsible for getting the Awesence info
// #[derive(Debug)] pub struct UpdateAwesense(pub(crate) AwesenseDataSet);
#[derive(Debug)] pub struct InitializeAwesense();

// This creates the enum with all the different types of messages are actor can receive including the default ones
define_actor_msg_set! { pub AwesenseImportActorMsg =  InitializeAwesense | ExecSnapshotActionAwesense | TraceDetailsAwesense }

/// this basically provides a message interface around an encapsulated, async updated OasisStore
#[derive(Debug)]
pub struct AwesenseActor<I, U> 
    where 
        U: DataAction<Vec<GridElement>>,
        I: DataRefAction<Vec<GridElement>>
{
    pub data_store: AwesenseDataSet,
    init_action: I,
    update_action: U
}

impl <I, U> AwesenseActor<I, U> 
    where 
        U: DataAction<Vec<GridElement>>,
        I: DataRefAction<Vec<GridElement>>
{
    pub async fn new (init_action: I, update_action: U, connection_info: AwesenseSqlInfo, grids: Vec<&str>) -> Self {
        let data_store = AwesenseDataSet::new(connection_info, grids).await.expect("failed to create AwesenseDataSet");
        AwesenseActor{data_store, init_action, update_action}
    }

    pub async fn init (&mut self) -> Result<()> {
        debug!("Init function of AwesenceActor");
        let _ = self.init_action.execute(&self.data_store.grid_elements).await;
        Ok(())
    }

    // pub async fn update (&mut self, new_dataset: AwesenseDataSet) -> Result<()> {
    //     debug!("update on OasisActor");
    //     //TODO  We don't want to clone this, refactor later
    //     self.data_store = new_dataset.clone();
    //     // TODO should there be handling for a failed update execute?
    //     let _ = self.update_action.execute(new_dataset).await;
    //     Ok(())
    // }
}

impl_actor! { match msg for Actor<AwesenseActor<I, U>, AwesenseImportActorMsg> 
    where 
        U: DataAction<Vec<GridElement>> + Sync,
        I: DataRefAction<Vec<GridElement>> + Sync
    as
    _Start_ => cont! { 
        // Just loading in static data for now, should be replaced with a data importer if
        // we expect the data to ever change.
        let hself = self.hself.clone();
        // let starting_data = OasisDataSet::from_csv("OASIS_Power_Data.csv", true).expect("Error getting the Oasis data from the CSV");
        // println!("Got the csv data in Oasis Start");
        let _ = hself.send_msg( InitializeAwesense()).await;
    }

    ExecSnapshotActionAwesense => cont! { 
        println!("Awesense Exec SnapShoot Action");
        let _ = msg.0.execute( &self.data_store.grid_elements ).await; 
    }

    TraceDetailsAwesense => cont! { 
        println!("Awesense Trace Details");
        let trace_elements = &self.data_store.get_trace_elements(&msg.1).await;
        let _ = msg.0.execute( trace_elements ).await; 
    }

    InitializeAwesense => cont! { 
        println!("Awesense Initialize on awesenseActor");
        // should anything happen is <Err> comes back from init? 
        let _ = self.init().await; 
    }

    // UpdateAwesense => cont! { let _ = self.update(msg.0).await; }
    
    // Add into code to terminate any actors that need to safely shutdown here
    _Terminate_ => stop! { }
}
