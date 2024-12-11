#![allow(unused_variables)]

use std::{net::SocketAddr,any::type_name};
use async_trait::async_trait;

use serde::{Serialize,Deserialize};

use odin_build::prelude::*;
use odin_actor::prelude::*;
use odin_server::prelude::*;
use odin_cesium::ImgLayerService;

use crate::{load_asset, GridElement, TraceDetailsAwesense};
use crate::awesense_actor::{ExecSnapshotActionAwesense, AwesenseImportActorMsg};

//--- aux types for creating JSON messages

// #[derive(Debug,Serialize,Deserialize)]
// #[serde(rename_all(serialize = "camelCase"))]
// pub struct AwesenseWebInfo {
//     pub connection_info: AwesenseSqlInfo,
//     pub grid_info: Vec<Grid>,
// }

#[derive(Debug)]
pub struct AwesenseWebService {
    pub hupdater: ActorHandle<AwesenseImportActorMsg>,
}

#[derive(Deserialize, Debug)]
pub struct TraceMessage {
    pub trace_name: String,
    pub grid_element_id: String,
}

//--- the SpaService
impl AwesenseWebService {
    pub fn new( hupdater: ActorHandle<AwesenseImportActorMsg>) -> Self { 
        Self { hupdater }
    }
}

#[async_trait]
impl SpaService for AwesenseWebService {
    fn add_dependencies (&self, spa_builder: SpaServiceList) -> SpaServiceList {
        spa_builder
            .add( build_service!( ImgLayerService::new()))
    }

    fn add_components (&self, spa: &mut SpaComponents) -> OdinServerResult<()>  {
        spa.add_assets( self_crate!(), load_asset);
        spa.add_module( asset_uri!("awesense_config.js"));
        spa.add_module( asset_uri!( "awesense_demo.js" ));
        spa.add_css( asset_uri!( "awesense_demo.css" ));

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
            debug!("Awesense service got a data available message from {}, of type: {}, with connections: {}", sender_id, data_type, has_connections.to_string());
            if data_type == type_name::<Vec<GridElement>>() {
                if has_connections {
                    let action = dyn_dataref_action!( let hself: ActorHandle<SpaServerMsg> = hself.clone() => |store: &Vec<GridElement>| {
                        let awesense_element_list = store;
                        let data = ws_msg!( "bcit_smart/awesense_demo.js", awesense_element_list).to_json()?;
                        debug!("Data available to be sent to all clients: {}", data);
                        hself.try_send_msg( BroadcastWsMsg{data})?;
                        Ok(())
                    });
                    hupdater.send_msg( ExecSnapshotActionAwesense(action)).await?;
                }
                is_our_data = true;
            }
        }

        Ok(is_our_data)
    }

    async fn init_connection (&mut self, hself: &ActorHandle<SpaServerMsg>, is_data_available: bool, conn: &mut SpaConnection) -> OdinServerResult<()> {
        println!("Init Connection for the Awesense service: Data available: {}", is_data_available);
        let initial_message = "Awesense Service initializing a connection";
        let msg = ws_msg!( "bcit_smart/awesense_demo.js", initial_message).to_json()?;
        conn.send(msg).await?;

        if is_data_available {
            let remote_addr = conn.remote_addr;
            let hupdater = &self.hupdater;

            let action = dyn_dataref_action!(
                let hself: ActorHandle<SpaServerMsg> = hself.clone(),
                let remote_addr: SocketAddr = remote_addr  => |store: &Vec<GridElement>| {
                    let awesense_element_list = store;
                    let data = ws_msg!( "bcit_smart/awesense_demo.js", awesense_element_list).to_json()?;
                    debug!("Init Connection data to be sent to all clients: {}", data);
                    hself.try_send_msg( BroadcastWsMsg{data})?;
                    Ok(())
            });
            hupdater.send_msg( ExecSnapshotActionAwesense(action)).await?;
        }

        Ok(())
    }

    async fn handle_ws_msg (&mut self, 
        hself: &ActorHandle<SpaServerMsg>, remote_addr: &SocketAddr, ws_msg_parts: &WsMsgParts
    ) -> OdinServerResult<WsMsgReaction> {
        debug!("AWESENSE Web Actor received WS msg: {:?}", ws_msg_parts.ws_msg);

        if ws_msg_parts.msg_type == "trace_request" {
            let trace_message: TraceMessage = serde_json::from_str(&ws_msg_parts.payload)?;
            let trace_name = &trace_message.trace_name;
            let grid_element_id = &trace_message.grid_element_id;

            // let response_message = format!("AWESENSE_WEB actor recieved WS message: traceName: {}, gridElementId: {}", trace_name, grid_element_id);
            let hupdater = &self.hupdater;

            let action = dyn_dataref_action!(
                let hself: ActorHandle<SpaServerMsg> = hself.clone(),
                let remote_addr: SocketAddr = remote_addr.clone() => |store: &Vec<GridElement>| {
                    let awesense_trace_response = store;
                    let data = ws_msg!( "bcit_smart/awesense_demo.js", awesense_trace_response).to_json()?;
                    debug!("Data to be sent to single client of trace: {}", data);
                    hself.try_send_msg( SendWsMsg{remote_addr: *remote_addr, data})?;
                    Ok(())
                });

            hupdater.send_msg( TraceDetailsAwesense(action, trace_message)).await?;
        }
        Ok( WsMsgReaction::None )
    }
}
