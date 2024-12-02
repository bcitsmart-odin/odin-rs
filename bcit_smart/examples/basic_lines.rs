use std::any::type_name;

use odin_actor::prelude::*;
use odin_server::prelude::*;

use odin_build;
use odin_actor::ActorHandle;

pub use bcit_smart::errors::*;

pub use bcit_smart::web::*;

pub use bcit_smart::actor::*;

pub use bcit_smart::live_importer::*;

pub struct TestImageService {}

#[async_trait::async_trait]
impl SpaService for TestImageService {

    fn add_dependencies (&self, spa_builder: SpaServiceList) -> SpaServiceList {
        spa_builder.add( build_service!( UiService::new()))
    }

    fn add_components (&self, spa: &mut SpaComponents) -> OdinServerResult<()> {
        spa.add_assets( self_crate!(), odin_server::load_asset);
        let odin_server = "odin_server";
        spa.add_module( asset_uri!(odin_server, "ui_windows.js"));
        spa.add_module( asset_uri!(odin_server, "test_image.js"));
        Ok(())
    }
}

run_actor_system!( actor_system => {
    //--- (1) set up PowerLines data source handle
    // Have to create a preactor handle to the PowerLine actor so that it can be handed Spa Actor before it is created.
    let preactor_handle_powerline = PreActorHandle::new ( &actor_system, "powerline", 8);

    // Unneeded info ATM, leaving for future extensibility
    let powerline_info = PowerLineInfo { line_id: 1, name: "testline".into(), description: "test description".into(), show: true };

    // Struct that holds info about PowerLine actor and a handle to the actor
    // we want to hold the handle and not the actor itself but maybe the info can be included in the Actor?
    let powerline_source = PowerLineSource::new( powerline_info, preactor_handle_powerline.to_actor_handle() );

    //--- (2) spawn the server actor
    let hserver = spawn_actor!( actor_system, "server", SpaServer::new(
        odin_server::load_config("spa_server.ron")?,
        "live",
        SpaServiceList::new()
        // Create a service here
            .add( build_service!( TestImageService{} )) // Currently having problems with asset files not being copied properly, if this is second PowerLineService won't work.
            .add( build_service!( PowerLineService::new(vec![powerline_source])) )
    ))?;

    //--- (3) spawn the data source actors we did set up in (1) 
    // should swap this to .ron file later
    let powerline_importer_config = LivePowerLineImporterConfig { 
        pow_id: 1,
        source: "data.json".into(),
        keep_files: true,   
        cleanup_interval: minutes(15),
        max_age: hours(3) 
    };
    let _hpowerline = spawn_powerline_updater( &mut actor_system, "powerline", preactor_handle_powerline, powerline_importer_config, &hserver)?;

    Ok(())
});

fn spawn_powerline_updater (
    actor_system: &mut ActorSystem,
    name: &'static str, 
    pre_handle: PreActorHandle<PowerLineImportActorMsg>, //  no importer for now
    config: LivePowerLineImporterConfig, // no config for now
    hserver: &ActorHandle<SpaServerMsg> // should handle sending the ws messages to frontend
) -> OdinActorResult<ActorHandle<PowerLineImportActorMsg>> {
    spawn_pre_actor!( actor_system, pre_handle,  PowerLineActor::new(
        // actor::load_config( "powerline.ron")?, // No config for now if it is added can go here 
        LivePowerLineImporter::new(config), // This would be a struct that handles getting powerline data
        dataref_action!( let hserver: ActorHandle<SpaServerMsg> = hserver.clone(), let name: &'static str = name => |_store:&Vec<PowerLineSet>| {
            println!("This should be executed by the init action");
            Ok( hserver.try_send_msg( DataAvailable{ sender_id: name, data_type: type_name::<Vec<PowerLineSet>>()} )? )
        }),
        data_action!( let hserver: ActorHandle<SpaServerMsg> = hserver.clone() => |powerlines:PowerLineSet| {
            let data = ws_msg!("bcit_smart/bcit_smart.js",powerlines).to_json()?;
            Ok( hserver.try_send_msg( BroadcastWsMsg{data})? )
        }),
    ))
}