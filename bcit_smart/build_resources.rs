use odin_build;
use std::process::Command;

/// common build script for crates that provide configs and assets
fn main () {
    // Specify the path to the directory containing .ts files
    let ts_dir = ".";
    let out_dir = "./assets";

    // Run the TypeScript compiler
    println!("cargo:rerun-if-changed={}", ts_dir);
    let status = Command::new("tsc")
        .args(&["--outDir", out_dir, "--project", ts_dir])
        .status()
        .expect("Failed to compile TypeScript files");

    assert!(status.success(), "TypeScript compilation failed");


    odin_build::init_build();
    if let Err(e) = odin_build::create_config_data() { panic!("failed to create config_data: {e}") }
    if let Err(e) = odin_build::create_asset_data() { panic!("failed to create asset_data: {e}") }
}
