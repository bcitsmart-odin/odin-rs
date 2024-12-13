use std::any::type_name;

use odin_actor::prelude::*;
use odin_server::prelude::*;

use odin_build;
use odin_actor::ActorHandle;

pub use bcit_smart::errors::*;

pub use bcit_smart::basic_web::*;

pub use bcit_smart::basic_actor::*;

pub use bcit_smart::basic_live_importer::*;

pub use bcit_smart::awesense_actor::*;
pub use bcit_smart::awesense_web::*;

pub use bcit_smart::{AwesenseSqlInfo, GridElement};

use tracing::Level;

use tokio;
use anyhow;

#[tokio::main]
async fn main ()->anyhow::Result<()> {
    odin_build::set_bin_context!();
    let mut actor_system = ActorSystem::new("main");
    actor_system.request_termination_on_ctrlc();

    // Customized debug messages because some of them were pages long when sending larger messages
    // bcit_smart::create_customized_tracing_subscriber(None); // use this one if you want to use the RUST_LOG env variable
    bcit_smart::create_customized_tracing_subscriber(Some(Level::DEBUG));

    let sql_config: AwesenseSqlInfo = bcit_smart::load_config("awesense_sql_config.ron")?;

    let preactor_handle_awesense = PreActorHandle::new ( &actor_system, "awesense", 8);

    let awesense_web_service = AwesenseWebService::new( preactor_handle_awesense.to_actor_handle() );
  
    //--- (2) spawn the server actor
    let hserver = spawn_actor!( actor_system, "server", SpaServer::new(
        odin_server::load_config("spa_server.ron")?,
        "live",
        SpaServiceList::new()
        // Create a service here
            .add( build_service!( => awesense_web_service ) )
    ))?;

    let _h_awesense_actor = spawn_pre_actor!( actor_system, preactor_handle_awesense, AwesenseActor::new(
        dataref_action!( let hserver: ActorHandle<SpaServerMsg> = hserver.clone() => |_store:&Vec<GridElement>| {
            println!("Awesense! This should be executed by the init action");
            Ok( hserver.try_send_msg( DataAvailable{ sender_id: "awesense", data_type: type_name::<Vec<GridElement>>()} )? )
        }),
        data_action!( let hserver: ActorHandle<SpaServerMsg> = hserver.clone() => |awesense_data:Vec<GridElement>| {
            println!("Awesense! This should be executed by the update action");
            let data = ws_msg!("bcit_smart/awesense_demo.js", awesense_data).to_json()?;
            Ok( hserver.try_send_msg( BroadcastWsMsg{data})? )
        }),
        sql_config,
        vec!["awefice"]
    ).await)?;

    actor_system.timeout_start_all(secs(2)).await?;
    actor_system.process_requests().await?;

    Ok(())
}
