/*
 * Copyright © 2024, United States Government, as represented by the Administrator of 
 * the National Aeronautics and Space Administration. All rights reserved.
 *
 * The “ODIN” software is licensed under the Apache License, Version 2.0 (the "License"); 
 * you may not use this file except in compliance with the License. You may obtain a copy 
 * of the License at http://www.apache.org/licenses/LICENSE-2.0.
 *
 * Unless required by applicable law or agreed to in writing, software distributed under
 * the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND,
 * either express or implied. See the License for the specific language governing permissions
 * and limitations under the License.
 */

 use std::os::unix::net::SocketAddr;

use tokio;
 use anyhow::Result;

use odin_build;
use odin_actor::prelude::*;
use odin_server::prelude::*;
use odin_sentinel::{SentinelStore,SentinelUpdate,LiveSentinelConnector,SentinelActor,load_config, web::SentinelService};


#[tokio::main]
async fn main()->Result<()> {
    odin_build::set_bin_context!();
    let mut actor_system = ActorSystem::new("main");
    let hsentinel = PreActorHandle::new( &actor_system, "updater", 8);

    let hserver = spawn_actor!( actor_system, "server", SpaServer::new(
        odin_server::load_config("spa_server.ron")?,
        "sentinels",
        SpaServiceListBuilder::new()
            .add( build_service!( SentinelService{})) // this automatically includes Cesium and UI services
            .build()
    ))?;

    
    let hsentinel = spawn_pre_actor!( actor_system, hsentinel, SentinelActor::new(
        LiveSentinelConnector::new( load_config( "sentinel.ron")?), 
        bi_dataref_action!( hserver.clone(): ActorHandle<SpaServerMsg> => |data:&SentinelStore, remote_addr:SocketAddr| {
            let data = data.to_json(false)?;
            hserver.try_send_msg( SendWsMsg{remote_addr,data})
        }),
        data_action!( hserver: ActorHandle<SpaServerMsg> => |data:SentinelUpdate| {
            let data = data.to_json()?;
            hserver.try_send_msg( BroadcastWsMsg{data})
        }),
    ))?;
    

    actor_system.timeout_start_all(secs(2)).await?;
    actor_system.process_requests().await?;

    Ok(())
}