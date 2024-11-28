//! actors for powerline data
use odin_actor::prelude::*;
use odin_build::prelude::*;

use odin_common::geo::LatLon;

use std::{fmt::Debug, sync::Arc};
use serde::Serialize;
use chrono::{DateTime, Utc};
// use std::future::Future;
use futures::Future;

use crate::errors::*;

// If we need individual config for this Actor can go here
// #[derive(Serialize,Deserialize,Debug)]
// pub struct PowerLineConfig {
//     pub max_records: usize,
// }

// Gonna fake this part to start with
// This part would store info about the data source used to get info
#[derive(Debug,PartialEq,Clone)]
pub struct PowerLineData {
    pub pow_id: u32,
    // pub file: PathBuf,
    pub source: Arc<String>,
    pub date: DateTime<Utc>
}

/// This is the Struct to hold info about individual power lines that will draw on the map
#[derive(Debug,Clone, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct PowerLine {
    pub pow_id: u32,
    pub positions: Vec<LatLon>, // Should maybe be vec to draw complex lines
    pub time: String
    // pub source: Arc<String>, // don't duplicate (think about this later)
    // pub pixel_size: Length // might be good to add later
}

impl PowerLine {
    pub fn new (pow_id: u32, position: LatLon, position2: LatLon, time: String)->Self {
        PowerLine {
            pow_id,
            positions: vec![position, position2],
            time
        }
    }
}

/// This is a struct to hold info about a set of powerlines
/// would include functionality that deals with whole set
/// will be what we send to the web socket to be used in the front end
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct PowerLineSet {
    pub powerlines: Vec<PowerLine>,
}

impl PowerLineSet {
    pub fn new(powerlines: Vec<PowerLine>) -> Self {
        // any info we want to extract from whole set can happen here

        PowerLineSet {
            powerlines
        }
    }
    pub fn to_json_pretty (&self)->Result<String> {
        Ok(serde_json::to_string_pretty( &self )?)
    }
    pub fn to_json (&self)->Result<String> {
        Ok(serde_json::to_string( &self )?)
    }
}

/// external message to request action execution with the current PowerLineStore
#[derive(Debug)] pub struct ExecSnapshotAction(pub DynDataRefAction<Vec<PowerLineSet>>);

// internal messages sent by the Actor responsible for getting the PowerLine info
// These structs will be used to send messages and
// will be included in a enum of different types messages can be
#[derive(Debug)] pub struct Update(pub(crate) PowerLineSet);
#[derive(Debug)] pub struct Initialize(pub(crate) Vec<PowerLineSet>);
// #[derive(Debug)] pub struct ImportError(pub(crate) OdinGoesrError);

// This creates the enum with all the different types of messages are actor can receive
// The ones listed in right of the equal sign are the extra ones beyond the default that we are adding
// The name on the left side of the equal sign defines the name of the created enum 
define_actor_msg_set! { pub PowerLineImportActorMsg =  Initialize | Update | ExecSnapshotAction }

/// user part of the PowerLine import actor
/// this basically provides a message interface around an encapsulated, async updated PowerLineStore
#[derive(Debug)]
pub struct PowerLineActor<IMP, U, I> 
    where 
        IMP: PowerLineDataImporter + Send,
        U: DataAction<PowerLineSet>,
        I: DataRefAction<Vec<PowerLineSet>>
{
    powerline_store: Vec<PowerLineSet>,
    powerline_importer: IMP,
    init_action: I,
    update_action: U
}

impl <IMP, U, I> PowerLineActor<IMP, U, I> 
    where 
        IMP: PowerLineDataImporter + Send,
        U: DataAction<PowerLineSet>,
        I: DataRefAction<Vec<PowerLineSet>>
{
    pub fn new (powerline_importer: IMP, init_action: I, update_action: U) -> Self {
         // This can just be a vector for now, if need more complex storage later can change
        let powerline_store = vec![];
        PowerLineActor{powerline_store, powerline_importer, init_action, update_action}
    }

    pub async fn init (&mut self, init_powerlines: Vec<PowerLineSet>) -> Result<()> {
        println!("Init function of PowerLineActor");
        self.powerline_store = init_powerlines.clone(); // should think about this more we already have an empty array at start
        println!("{:?}", init_powerlines);
        // TODO should there be handling for a failed init execute?
        let _ = self.init_action.execute(&self.powerline_store).await;
        Ok(())
    }

    pub async fn update (&mut self, new_powerlines: PowerLineSet) -> Result<()> {
        println!("update on PowerLineActor");
        self.powerline_store.push(new_powerlines.clone());
        // TODO should there be handling for a failed update execute?
        let _ = self.update_action.execute(new_powerlines).await;
        Ok(())
    }
}

impl_actor! { match msg for Actor< PowerLineActor<IMP, U, I>, PowerLineImportActorMsg> 
    where 
        IMP: PowerLineDataImporter + Send + Sync,
        I: DataRefAction<Vec<PowerLineSet>> + Sync,
        U: DataAction<PowerLineSet> + Sync
    as
    _Start_ => cont! { 
        // We should be starting the actor responsible for getting PowerLine info here that will
        // then start sending this actor updates when it has data.
        println!("Start on PowerLineActor");
        let hself = self.hself.clone();
        // TODO consider what to do if start fails or if it can fail
        let _ = self.powerline_importer.start(hself).await;
    }

    ExecSnapshotAction => cont! { 
        println!("Exec SnapShoot Action");
        let _ = msg.0.execute( &self.powerline_store).await; 
    } // Ignoring this for now

    Initialize => cont! { 
        println!("Initialize on powerlineactor");
        // should anything happen is <Err> comes back from init? 
        let _ = self.init(msg.0).await; 
    }

    Update => cont! { let _ = self.update(msg.0).await; }

    // ImportError => cont! { error!("{:?}", msg.0); } // think about errors later
    
    _Terminate_ => stop! { 
        self.powerline_importer.terminate();   // When we setup an import struct we may have to stop it safely
    }
}

/// abstraction for the data acquisition mechanism used by the GoesRImportActor
/// impl objects are used as GoesRImportActor constructor arguments. It is Ok to panic in the instantiation
pub trait PowerLineDataImporter {
    fn start (&mut self, hself: ActorHandle<PowerLineImportActorMsg>) -> impl Future<Output=Result<()>> + Send;
    fn terminate (&mut self);
}