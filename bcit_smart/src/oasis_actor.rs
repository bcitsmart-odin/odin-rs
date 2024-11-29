use crate::*;
use odin_build::decompress_vec;

use std::{fmt::Debug, sync::Arc, time::Duration};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use csv::ReaderBuilder;
use std::io::Cursor;

// This so be info about the total OASIS dataset
#[derive(Debug,PartialEq,Clone)]
pub struct OasisDataSettings {
    pub interval: Duration,
    pub source: Arc<String>,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>
}

/// This is the Struct to hold info about individual row of data from the OASIS dataset
#[derive(Debug,Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct OasisDataRow {
    #[serde(skip_deserializing)]
    pub row_num: u32, // Not part of the CSV; added manually
    #[serde(rename = "timestamp")]
    pub time_stamp: String,
    // pub time_stamp: DateTime<Utc>, // Can't get this to work just using String for now
    #[serde(rename = "PV.DC Power")]
    pub pv_dc_power: Option<f64>,
    #[serde(rename = "BESS.DC Power")]
    pub bess_dc_power: Option<f64>,
    #[serde(rename = "Battery Group 1.DC Power")]
    pub battery_group_1_dc_power: Option<f64>,
    #[serde(rename = "Battery Group 2.DC Power")]
    pub battery_group_2_dc_power: Option<f64>,
    #[serde(rename = "Battery Group 3.DC Power")]
    pub battery_group_3_dc_power: Option<f64>,
    #[serde(rename = "Battery Group 4.DC Power")]
    pub battery_group_4_dc_power: Option<f64>,
    #[serde(rename = "Inverter.Active Power")]
    pub inverter_active_power: Option<f64>,
    #[serde(rename = "L2 7650 Meter.Active Power")]
    pub l2_7650_meter_active_power: Option<f64>,
    #[serde(rename = "OASIS POI.Active Power")]
    pub oasis_poi_active_power: Option<f64>,
}								

/// would include functionality that deals with whole set
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct OasisDataSet {
    pub data_rows: Vec<OasisDataRow>,
}

impl OasisDataSet {
    pub fn new(data_rows: Vec<OasisDataRow>) -> Self {
        // any info we want to extract from whole set can happen here

        OasisDataSet {
            data_rows
        }
    }

    pub fn from_csv(asset_file_name: &str, is_excel_export: bool) -> Result<OasisDataSet> {
        println!("Starting from CSV");
        let data = load_asset(asset_file_name).expect(&format!("Didn't open file, {}", asset_file_name));
        let decompressed_data = decompress_vec(&data).expect(&format!("Didn't decompress file, {}", asset_file_name));
        let mut csv_content = String::from_utf8(decompressed_data)?;
        // println!("Raw decompressed data: {:?}", String::from_utf8_lossy(&decompressed_data[..1000]));
        println!("csv content: {}", &csv_content[..1000]);

        // This file came from excel and needs to be cleaned up before it can work
        // Need to remove the = prefix on lines and possibly the quotes
        if is_excel_export {
            csv_content = csv_content
                .lines()
                .map(|line| {
                    if line.starts_with('=') {
                        &line[1..]
                    } else {
                        line
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
        }
        println!("csv content: {}", &csv_content[..1000]);

        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(Cursor::new(csv_content));

        let mut rows = Vec::new();
        for (idx, result) in rdr.deserialize().enumerate() {
            match result {
                Ok(record) => {
                    let mut record: OasisDataRow = record;
                    record.row_num = (idx + 1) as u32;
                    rows.push(record);
                }
                Err(err) => {
                    // Do we want to throw an error if any rows won't parse?
                    // eprintln!("Error parsing row {}: {:?}, raw row: {:?}", idx + 1, err, rdr.records().nth(idx));
                    // return Err(BcitSmartError::MiscError(format!(
                    //     "Error parsing row {}: {:?}",
                    //     idx + 1, err
                    // )));
                    eprintln!(
                        "Skipping malformed row {}: {:?}",
                        idx + 1,
                        err
                    );
                    continue;
                }
            }
        }

        Ok(OasisDataSet { data_rows: rows })
    }

    pub fn to_json_pretty (&self)->Result<String> {
        Ok(serde_json::to_string_pretty( &self )?)
    }
    pub fn to_json (&self)->Result<String> {
        Ok(serde_json::to_string( &self )?)
    }
}

/// external message to request action execution with the current OasisDataStore
#[derive(Debug)] pub struct ExecSnapshotActionOasis(pub DynDataRefAction<OasisDataSet>);

// internal messages sent by the Actor responsible for getting the Oasis info
#[derive(Debug)] pub struct UpdateOasis(pub(crate) OasisDataSet);
#[derive(Debug)] pub struct InitializeOasis(pub(crate) OasisDataSet);
// #[derive(Debug)] pub struct ImportError(pub(crate) OdinGoesrError);

// This creates the enum with all the different types of messages are actor can receive
// The ones listed in right of the equal sign are the extra ones beyond the default that we are adding
// The name on the left side of the equal sign defines the name of the created enum 
define_actor_msg_set! { pub OasisImportActorMsg =  InitializeOasis | UpdateOasis | ExecSnapshotActionOasis }

/// this basically provides a message interface around an encapsulated, async updated OasisStore
#[derive(Debug)]
pub struct OasisActor<I, U> 
    where 
        U: DataAction<OasisDataSet>,
        I: DataRefAction<OasisDataSet>
{
    data_store: OasisDataSet,
    init_action: I,
    update_action: U
}

impl <I, U> OasisActor<I, U> 
    where 
        U: DataAction<OasisDataSet>,
        I: DataRefAction<OasisDataSet>
{
    pub fn new (init_action: I, update_action: U) -> Self {
        // Think about when we want the dataset loaded in
        let data_store = OasisDataSet { data_rows: vec![] };
        OasisActor{data_store, init_action, update_action}
    }

    pub async fn init (&mut self, init_dataset: OasisDataSet) -> Result<()> {
        println!("Init function of OasisActor");
        self.data_store = init_dataset.clone(); // should think about this more we already have an empty array at start
        println!("{:?}", &init_dataset.data_rows[..10]);
        // TODO should there be handling for a failed init execute?
        let _ = self.init_action.execute(&self.data_store).await;
        Ok(())
    }

    pub async fn update (&mut self, new_dataset: OasisDataSet) -> Result<()> {
        println!("update on OasisActor");
        //TODO  We don't want to clone this, refactor later
        self.data_store = new_dataset.clone();
        // TODO should there be handling for a failed update execute?
        let _ = self.update_action.execute(new_dataset).await;
        Ok(())
    }
}

impl_actor! { match msg for Actor<OasisActor<I, U>, OasisImportActorMsg> 
    where 
        U: DataAction<OasisDataSet> + Sync,
        I: DataRefAction<OasisDataSet> + Sync
    as
    _Start_ => cont! { 
        // We should be starting the actor responsible for getting PowerLine info here that will
        // then start sending this actor updates when it has data.
        println!("Start on OasisActor");
        let hself = self.hself.clone();
        // self.data_store = OasisDataSet::from_csv("OASIS_Power_Data.csv", true);
        let starting_data = OasisDataSet::from_csv("OASIS_Power_Data.csv", true).expect("Couldn't get data");
        println!("Got the csv data in Oasis Start");
        let _ = hself.send_msg( InitializeOasis( starting_data )).await;
    }

    ExecSnapshotActionOasis => cont! { 
        println!("Oasis Exec SnapShoot Action");
        let _ = msg.0.execute( &self.data_store).await; 
    } // Ignoring this for now

    InitializeOasis => cont! { 
        println!("Oasis Initialize on oasisActor");
        // should anything happen is <Err> comes back from init? 
        let _ = self.init(msg.0).await; 
    }

    UpdateOasis => cont! { let _ = self.update(msg.0).await; }
    
    // Add into code to terminate any actors that need to safely shutdown here
    _Terminate_ => stop! { }
}
