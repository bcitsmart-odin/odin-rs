#![allow(unused)]

use std::{net::SocketAddr,any::type_name,fs};
use async_trait::async_trait;
use axum::{
    http::{Uri,StatusCode},
    body::Body,
    routing::{Router,get},
    extract::{Path as AxumPath},
    response::{Response,IntoResponse},
};
use serde::{Serialize,Deserialize};

use odin_build::prelude::*;
use odin_actor::prelude::*;
use odin_server::prelude::*;
use odin_cesium::ImgLayerService;

use crate::{load_asset, load_config};
use crate::actor::{ExecSnapshotAction, PowerLineImportActorMsg, PowerLineSet};

//--- aux types for creating JSON messages

#[derive(Debug,Serialize,Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct PowerLineInfo {
    pub line_id: u32,
    pub name: String,
    pub description: String,
    pub show: bool,
}

// Think about this name and this structs purpose more
pub struct PowerLineSource {
    pub info: PowerLineInfo,
    pub hupdater: ActorHandle<PowerLineImportActorMsg>
}

impl PowerLineSource {
    pub fn new( info: PowerLineInfo, hupdater: ActorHandle<PowerLineImportActorMsg>) -> Self { PowerLineSource { info, hupdater } }
}

//--- the SpaService

/// microservice for PowerLine data
pub struct PowerLineService {
    // if we only ever going to have 1 source should change this
    powerlines: Vec<PowerLineSource>
}

impl PowerLineService {
    pub fn new (powerlines: Vec<PowerLineSource> )-> Self { PowerLineService{ powerlines } }
}

#[async_trait]
impl SpaService for PowerLineService {
    fn add_dependencies (&self, spa_builder: SpaServiceList) -> SpaServiceList {
        spa_builder.add( build_service!( ImgLayerService::new()))
    }

    fn add_components (&self, spa: &mut SpaComponents) -> OdinServerResult<()>  {
        spa.add_assets( self_crate!(), load_asset);
        spa.add_module( asset_uri!("bcit_smart_config.js"));
        spa.add_module( asset_uri!( "bcit_smart.js" ));

        Ok(())
    }

    async fn data_available (
        &mut self,
        hself: &ActorHandle<SpaServerMsg>,
        has_connections: bool,
        sender_id: &str,
        data_type: &str
    ) -> OdinServerResult<bool> {
        let mut is_our_data = false;

        println!("data available function");

        let hupdater = &self.powerlines[0].hupdater;

        if (*hupdater.id == sender_id) {
            println!("BCIT SMART service got a data available message from {}, of type: {}, with connections: {}", sender_id, data_type, has_connections.to_string());
            if data_type == type_name::<Vec<PowerLineSet>>() {
                println!("Going to send a snapshot action");
                if has_connections {
                    let action = dyn_dataref_action!( hself.clone(): ActorHandle<SpaServerMsg> => |store: &Vec<PowerLineSet>| {
                        println!("Action send to snapshotAction being run");
                        for powerlines in store {
                            let data = ws_msg!( "bcit_smart/bcit_smart.js", powerlines).to_json()?;
                            println!("Data to be sent\n {}", data);
                            hself.try_send_msg( BroadcastWsMsg{data})?;
                        }
                        Ok(())
                    });
                    hupdater.send_msg( ExecSnapshotAction(action)).await?;
                    println!("Going to try to send message to the ws");
                }
                is_our_data = true;
            }
        }

        Ok(is_our_data)
    }

    async fn init_connection (&mut self, hself: &ActorHandle<SpaServerMsg>, is_data_available: bool, conn: &mut SpaConnection) -> OdinServerResult<()> {
        // let satellites: Vec<&GoesrSatelliteInfo> = self.satellites.iter().map( |s| &s.info).collect();
        println!("Init Connection for the power line service");
        let initial_message = "This is the powerline service initializing a connection";
        let msg = ws_msg!( "bcit_smart/bcit_smart.js", initial_message).to_json()?;
        conn.send(msg).await?;

        if is_data_available {
            let remote_addr = conn.remote_addr;
            let hupdater = &self.powerlines[0].hupdater;

            let action = dyn_dataref_action!( hself.clone(): ActorHandle<SpaServerMsg>, remote_addr: SocketAddr  => |store: &Vec<PowerLineSet>| {
                for powerlines in store {
                    let remote_addr = remote_addr.clone();
                    let data = ws_msg!( "bcit_smart/bcit_smart.js", powerlines).to_json()?;
                    hself.try_send_msg( SendWsMsg{remote_addr, data})?;
                }
                Ok(())
            });
            hupdater.send_msg( ExecSnapshotAction(action)).await?;
        }

        Ok(())
    }
}
