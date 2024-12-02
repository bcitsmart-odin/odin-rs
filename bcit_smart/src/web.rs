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
        spa_builder
            .add( build_service!( ImgLayerService::new()))
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
        let hupdater = &self.powerlines[0].hupdater;

        if (*hupdater.id == sender_id) {
            debug!("BCIT SMART service got a data available message from {}, of type: {}, with connections: {}", sender_id, data_type, has_connections.to_string());
            if data_type == type_name::<Vec<PowerLineSet>>() {
                if has_connections {
                    let action = dyn_dataref_action!( let hself: ActorHandle<SpaServerMsg> = hself.clone() => |store: &Vec<PowerLineSet>| {
                        for powerlines in store {
                            let data = ws_msg!( "bcit_smart/bcit_smart.js", powerlines).to_json()?;
                            debug!("Data to be sent to all clients: {}", data);
                            hself.try_send_msg( BroadcastWsMsg{data})?;
                        }
                        Ok(())
                    });
                    hupdater.send_msg( ExecSnapshotAction(action)).await?;
                }
                is_our_data = true;
            }
        }

        Ok(is_our_data)
    }

    async fn init_connection (&mut self, hself: &ActorHandle<SpaServerMsg>, is_data_available: bool, conn: &mut SpaConnection) -> OdinServerResult<()> {
        // let satellites: Vec<&GoesrSatelliteInfo> = self.satellites.iter().map( |s| &s.info).collect();
        debug!("Init Connection for the power line service");
        let initial_message = "This is the powerline service initializing a connection";
        let msg = ws_msg!( "bcit_smart/bcit_smart.js", initial_message).to_json()?;
        conn.send(msg).await?;

        if is_data_available {
            let remote_addr = conn.remote_addr;
            let hupdater = &self.powerlines[0].hupdater;

            let action = dyn_dataref_action!{ 
                let hself: ActorHandle<SpaServerMsg> = hself.clone(),
                let remote_addr: SocketAddr = remote_addr => 
                    |store: &Vec<PowerLineSet>| {
                        for powerlines in store {
                            let remote_addr = remote_addr.clone();
                            let data = ws_msg!( "bcit_smart/bcit_smart.js", powerlines).to_json()?;
                            hself.try_send_msg( SendWsMsg{remote_addr, data})?;
                        }
                    Ok(())
            }};
            hupdater.send_msg( ExecSnapshotAction(action)).await?;
        }

        Ok(())
    }

    async fn handle_ws_msg (&mut self, 
        hself: &ActorHandle<SpaServerMsg>, remote_addr: &SocketAddr, ws_msg_parts: &WsMsgParts
    ) -> OdinServerResult<WsMsgReaction> {
        debug!("Handling WS message for bcitsmart/web: {:?}", ws_msg_parts.ws_msg);

        // Send a message to the PowerLine actor
        let hupdater = &self.powerlines[0].hupdater;


        let response_message = "BCIT_SMART web actor recieved WS message";
        let response = ws_msg!( "bcit_smart/bcit_smart.js", response_message).to_json()?;
        Ok( WsMsgReaction::Broadcast(response) )
    }
}
