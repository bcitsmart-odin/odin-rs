use std::any::type_name;

use odin_actor::prelude::*;
use odin_server::prelude::*;

use odin_build;
use odin_actor::ActorHandle;

pub use bcit_smart::errors::*;

pub use bcit_smart::web::*;

pub use bcit_smart::actor::*;

pub use bcit_smart::live_importer::*;

pub use bcit_smart::oasis_actor::*;
pub use bcit_smart::oasis_web::*;

pub use bcit_smart::awesense_actor::*;
pub use bcit_smart::awesense_web::*;

pub use bcit_smart::{AwesenseSqlInfo, GridElement};

use odin_goesr::{
    LiveGoesrHotspotImporter, LiveGoesrHotspotImporterConfig,
    GoesrHotspotStore, GoesrHotspotSet, GoesrHotspotActor, GoesrHotspotImportActorMsg, GoesrSat, GoesrService
};

use tracing::Level;

use tokio;
use anyhow;

pub struct TestImageService {}

#[async_trait::async_trait]
impl SpaService for TestImageService {

    fn add_dependencies (&self, spa_builder: SpaServiceList) -> SpaServiceList {
        spa_builder.add( build_service!( UiService::new()))
    }

    fn add_components (&self, spa: &mut SpaComponents) -> OdinServerResult<()> {
        spa.add_assets( self_crate!(), odin_server::load_asset);
        spa.add_module( asset_uri!(odin_server, "ui_windows.js"));
        spa.add_module( asset_uri!(odin_server, "test_image.js"));
        Ok(())
    }
}

#[tokio::main]
async fn main ()->anyhow::Result<()> {
    odin_build::set_bin_context!();
    let mut actor_system = ActorSystem::new("main");
    actor_system.request_termination_on_ctrlc();

    // Customized debug messages because some of them were pages long when sending larger messages
    // bcit_smart::create_customized_tracing_subscriber(None); // use this one if you want to use the RUST_LOG env variable
    bcit_smart::create_customized_tracing_subscriber(Some(Level::DEBUG));

    //--- (1a) set up Awesense Actors
    let sql_config: AwesenseSqlInfo = bcit_smart::load_config("awesense_sql_config.ron")?;
    let preactor_handle_awesense = PreActorHandle::new ( &actor_system, "awesense", 8);
    let awesense_web_service = AwesenseWebService::new( preactor_handle_awesense.to_actor_handle() );

     //--- (1b) set up Oasis Actors
    let preactor_handle_oasis = PreActorHandle::new ( &actor_system, "oasis", 8);
    let oasis_info = OasisInfo { line_id: 1, name: "Oasis".into(), description: "Oasis description".into(), show: true };
    let oasis_web_service = OasisService::new( oasis_info, preactor_handle_oasis.to_actor_handle() );

    //--- (1c) set up Goesr Actors
    let hgoes18 = PreActorHandle::new( &actor_system, "goes18", 8);
    let goes18 = GoesrSat::new( odin_goesr::load_config("goes_18.ron")?, hgoes18.to_actor_handle());
    let hgoes16 = PreActorHandle::new( &actor_system, "goes16", 8);
    let goes16 = GoesrSat::new( odin_goesr::load_config("goes_16.ron")?, hgoes16.to_actor_handle());

    //--- (2) spawn the server actor
    let hserver = spawn_actor!( actor_system, "server", SpaServer::new(
        odin_server::load_config("spa_server.ron")?,
        "live",
        SpaServiceList::new()
        // Create a service here
            .add( build_service!( TestImageService{} )) // Currently having problems with asset files not being copied properly, if this is second PowerLineService won't work.
            .add( build_service!( oasis_web_service ) )
            .add( build_service!( awesense_web_service ) )
            .add( build_service!( GoesrService::new( vec![goes18,goes16])) )
    ))?;

    //--- (3) spawn the data source actors we did set up in (1)
        let _hawesense_actor = spawn_pre_actor!( actor_system, preactor_handle_awesense, AwesenseActor::new(
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

    let _hoasis_actor = spawn_pre_actor!( actor_system, preactor_handle_oasis, OasisActor::new(
        dataref_action!( let hserver: ActorHandle<SpaServerMsg> = hserver.clone() => |_store:&OasisDataSet| {
            println!("OASIS! This should be executed by the init action");
            Ok( hserver.try_send_msg( DataAvailable{ sender_id: "oasis", data_type: type_name::<OasisDataSet>()} )? )
        }),
        data_action!( let hserver: ActorHandle<SpaServerMsg> = hserver.clone() => |oasis_data:OasisDataSet| {
            println!("OASIS! This should be executed by the update action");
            let data = ws_msg!("bcit_smart/bcit_smart.js",oasis_data).to_json()?;
            Ok( hserver.try_send_msg( BroadcastWsMsg{data})? )
        }),
    ))?;

    let _hgoes18 = spawn_goesr_updater( &mut actor_system, "goes18", hgoes18, odin_goesr::load_config( "goes_18_fdcc.ron")?, &hserver)?;
    let _hgoes16 = spawn_goesr_updater( &mut actor_system, "goes16", hgoes16, odin_goesr::load_config( "goes_16_fdcc.ron")?, &hserver)?;

    actor_system.timeout_start_all(secs(2)).await?;
    actor_system.process_requests().await?;

    Ok(())
}

fn spawn_goesr_updater (
    actor_system: &mut ActorSystem,
    name: &'static str, 
    pre_handle: PreActorHandle<GoesrHotspotImportActorMsg>, 
    config: LiveGoesrHotspotImporterConfig,
    hserver: &ActorHandle<SpaServerMsg>
) ->OdinActorResult<ActorHandle<GoesrHotspotImportActorMsg>> {
    spawn_pre_actor!( actor_system, pre_handle,  GoesrHotspotActor::new(
        odin_goesr::load_config( "goesr.ron")?, 
        LiveGoesrHotspotImporter::new( config),
        dataref_action!{
            let hserver: ActorHandle<SpaServerMsg> = hserver.clone(), 
            let name: &'static str = name => 
            |_store:&GoesrHotspotStore| {
                Ok( hserver.try_send_msg( DataAvailable{ sender_id: name, data_type: type_name::<GoesrHotspotStore>()} )? )
            }
        },
        data_action!( let hserver: ActorHandle<SpaServerMsg> = hserver.clone() => |hotspots:GoesrHotspotSet| {
            //let data = ws_msg!("odin_goesr/odin_goesr.js",hotspots).to_json()?;
            let data = WsMsg::json( GoesrService::mod_path(), "hotspots", hotspots)?;
            Ok( hserver.try_send_msg( BroadcastWsMsg{data})? )
        }),
    ))
}