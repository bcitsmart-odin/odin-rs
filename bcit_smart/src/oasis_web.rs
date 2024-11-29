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
use odin_server::spa::WebSocketMessage;
use odin_cesium::ImgLayerService;

use crate::{load_asset, load_config};
use crate::oasis_actor::{ExecSnapshotActionOasis, OasisImportActorMsg, OasisDataSet};

//--- aux types for creating JSON messages

#[derive(Debug,Serialize,Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct OasisInfo {
    pub line_id: u32,
    pub name: String,
    pub description: String,
    pub show: bool,
}

// Think about this name and this structs purpose more
pub struct OasisService {
    pub info: OasisInfo,
    pub hupdater: ActorHandle<OasisImportActorMsg>
}

//--- the SpaService
impl OasisService {
    pub fn new( info: OasisInfo, hupdater: ActorHandle<OasisImportActorMsg>) -> Self { OasisService { info, hupdater } }
}

#[async_trait]
impl SpaService for OasisService {
    async fn add_dependencies (&self, spa_builder: SpaServiceList) -> SpaServiceList {
        spa_builder
            .add( build_service!( ImgLayerService::new())).await
    }

    fn add_components (&self, spa: &mut SpaComponents) -> OdinServerResult<()>  {
        spa.add_assets( self_crate!(), load_asset);
        spa.add_module( asset_uri!("oasis_config.js"));
        spa.add_module( asset_uri!( "oasis_points.js" ));
        spa.add_script( "https://cdn.jsdelivr.net/npm/chart.js" );
        spa.add_script( "https://cdn.jsdelivr.net/npm/moment@^2" );
        spa.add_script( "https://cdn.jsdelivr.net/npm/chartjs-adapter-moment@^1" );
        spa.add_script( "https://cdn.jsdelivr.net/npm/hammerjs@2.0.8" );
        spa.add_script( "https://cdn.jsdelivr.net/npm/chartjs-plugin-zoom" );

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

        println!("Oasis SPA Service data available function");

        let hupdater = &self.hupdater;

        if (*hupdater.id == sender_id) {
            println!("Oasis service got a data available message from {}, of type: {}, with connections: {}", sender_id, data_type, has_connections.to_string());
            if data_type == type_name::<OasisDataSet>() {
                println!("Going to send a snapshot action");
                if has_connections {
                    let action = dyn_dataref_action!( hself.clone(): ActorHandle<SpaServerMsg> => |store: &OasisDataSet| {
                        println!("Action send to snapshotAction being run");
                        let oasis_data = &store.data_rows;
                        let data = ws_msg!( "bcit_smart/oasis_points.js", oasis_data).to_json()?;
                        println!("Data to be sent\n {}", data);
                        hself.try_send_msg( BroadcastWsMsg{data})?;
                        Ok(())
                    });
                    hupdater.send_msg( ExecSnapshotActionOasis(action)).await?;
                    println!("Going to try to send message to the ws");
                }
                is_our_data = true;
            }
        }

        Ok(is_our_data)
    }

    async fn init_connection (&mut self, hself: &ActorHandle<SpaServerMsg>, is_data_available: bool, conn: &mut SpaConnection) -> OdinServerResult<()> {
        println!("Init Connection for the Oasis service");
        let initial_message = "Oasis Service initializing a connection";
        let msg = ws_msg!( "bcit_smart/oasis_points.js", initial_message).to_json()?;
        conn.send(msg).await?;

        if is_data_available {
            let remote_addr = conn.remote_addr;
            let hupdater = &self.hupdater;

            let action = dyn_dataref_action!( hself.clone(): ActorHandle<SpaServerMsg>, remote_addr: SocketAddr  => |store: &OasisDataSet| {
                let oasis_data = &store.data_rows;
                let data = ws_msg!( "bcit_smart/oasis_points.js", oasis_data).to_json()?;
                println!("Init Connection data to be sent\n {}", data);
                hself.try_send_msg( BroadcastWsMsg{data})?;
                Ok(())
            });
            hupdater.send_msg( ExecSnapshotActionOasis(action)).await?;
        }

        Ok(())
    }

    async fn handle_incoming_ws_msg (&mut self, msg: String) -> OdinServerResult<()> {
        println!("Handling incoming ws msg, {}", msg);
        match serde_json::from_str::<WebSocketMessage>(&msg) {
            Ok(parsed_msg) => {
                let target_module = parsed_msg.module;
                let payload = parsed_msg.payload;

                println!("Oasis target module: {}", target_module);
                println!("Oasis payload: {}", payload);
            }
            Err(e) => println!("Failed to parse WebSocket message: {:?}", e),
        }
        Ok(())
    }
}
