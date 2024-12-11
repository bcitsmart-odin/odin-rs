use std::{net::SocketAddr,any::type_name};
use async_trait::async_trait;
// use axum::{
    // http::{Uri,StatusCode},
    // body::Body,
    // routing::{Router,get},
    // extract::{Path as AxumPath},
    // response::{Response,IntoResponse},
// };
use serde::{Serialize,Deserialize};

use odin_build::prelude::*;
use odin_actor::prelude::*;
use odin_server::prelude::*;
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
    fn add_dependencies (&self, spa_builder: SpaServiceList) -> SpaServiceList {
        spa_builder
            .add( build_service!( ImgLayerService::new()))
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
        let hupdater = &self.hupdater;

        if *hupdater.id == sender_id {
            debug!("Oasis service got a data available message from {}, of type: {}, with connections: {}", sender_id, data_type, has_connections.to_string());
            if data_type == type_name::<OasisDataSet>() {
                if has_connections {
                    let action = dyn_dataref_action!( let hself: ActorHandle<SpaServerMsg> = hself.clone() => |store: &OasisDataSet| {
                        let oasis_data = &store.data_rows;
                        let data = ws_msg!( "bcit_smart/oasis_points.js", oasis_data).to_json()?;
                        debug!("Data available to be sent to all clients: {}", data);
                        hself.try_send_msg( BroadcastWsMsg{data})?;
                        Ok(())
                    });
                    hupdater.send_msg( ExecSnapshotActionOasis(action)).await?;
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

            let action = dyn_dataref_action!(
                let hself: ActorHandle<SpaServerMsg> = hself.clone(),
                let _remote_addr: SocketAddr = remote_addr  => |store: &OasisDataSet| {
                    let oasis_data = &store.data_rows;
                    let data = ws_msg!( "bcit_smart/oasis_points.js", oasis_data).to_json()?;
                    debug!("Init Connection data to be sent to all clients: {}", data);
                    hself.try_send_msg( BroadcastWsMsg{data})?;
                    Ok(())
            });
            hupdater.send_msg( ExecSnapshotActionOasis(action)).await?;
        }

        Ok(())
    }

    async fn handle_ws_msg (&mut self, 
        _hself: &ActorHandle<SpaServerMsg>, _remote_addr: &SocketAddr, ws_msg_parts: &WsMsgParts
    ) -> OdinServerResult<WsMsgReaction> {
        debug!("Oasis Web Actor received WS msg: {:?}", ws_msg_parts.ws_msg);
        let _hupdater = &self.hupdater;

        let response_message = "OASIS_WEB actor recieved WS message";
        let response = ws_msg!( "bcit_smart/oasis_points.js", response_message).to_json()?;
        Ok( WsMsgReaction::Broadcast(response) )
    }
}
