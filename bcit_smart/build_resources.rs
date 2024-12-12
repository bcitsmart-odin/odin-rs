use odin_build;
use std::process::Command;
use std::env;

/// common build script for crates that provide configs and assets
fn main () {
    
    // Run the TypeScript compiler
    let should_compile_ts = env::var("COMPILE_TS").is_ok();
    // let should_compile_ts = true; // if the env doesn't work
    
    // Specify the path to the directory containing .ts files
    if should_compile_ts {
        let ts_dir = ".";
        let out_dir = "./assets";

        println!("cargo:rerun-if-changed={}", ts_dir);
        let status = Command::new("tsc")
            .args(&["--outDir", out_dir, "--project", ts_dir])
            .status()
            .expect("Failed to compile TypeScript files");
    
        assert!(status.success(), "TypeScript compilation failed");
    } else {
        println!("Skipping TypeScript compilation; COMPILE_TS not set.");
    }


    odin_build::init_build();
    if let Err(e) = odin_build::create_config_data() { panic!("failed to create config_data: {e}") }
    if let Err(e) = odin_build::create_asset_data() { panic!("failed to create asset_data: {e}") }
}
