use crate::*;
use odin_common::fs::{ensure_writable_dir, remove_old_files};
use std::path::PathBuf;
use std::time::Duration;
use std::sync::Arc;

use serde::Deserialize;

use std::io::Read;
use flate2::read::GzDecoder;
use rand::Rng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;

use odin_common::geo::LatLon;

#[derive(Deserialize,Debug,Clone)]
struct PowerLineDataPoint {
    time: String,
    coordinates: Vec<f64>,
}

/// configuration for live powerline import
#[derive(Deserialize,Debug,Clone)]
pub struct LivePowerLineImporterConfig {
    pub pow_id: u32,
    pub source: String, // gonna just store dummy data in a file
    pub keep_files: bool,
    pub cleanup_interval: Duration,
    pub max_age: Duration,
}

/// the structure representing objects to collect and announce availability of powerline data
/// 
/// (REQ) instance should check availability of new data sets on a guaranteed time interval
/// (REQ) instance should not miss any available data set once initialized 
#[derive(Debug)]
pub struct LivePowerLineImporter {
    config: LivePowerLineImporterConfig,
    cache_dir: Arc<PathBuf>,

    /// values set during initialization
    import_task: Option<AbortHandle>,
    file_cleanup_task: Option<AbortHandle>,
}

impl LivePowerLineImporter {
    pub fn new (config: LivePowerLineImporterConfig) -> Self {
        let cache_dir = Arc::new( odin_build::cache_dir().join("bcit"));
        ensure_writable_dir(cache_dir.as_ref()).unwrap(); // Ok to panic - this is a toplevel application object

        LivePowerLineImporter{ config, cache_dir, import_task:None, file_cleanup_task:None }
    }

    async fn initialize  (&mut self, hself: ActorHandle<PowerLineImportActorMsg>) -> Result<()> { 
        self.import_task = Some( self.spawn_import_task(hself)? );
        self.file_cleanup_task = Some( self.spawn_file_cleanup_task()? );
        Ok(())
    }

    fn spawn_import_task(&mut self, hself: ActorHandle<PowerLineImportActorMsg>) -> Result<AbortHandle> { 
        println!("spawning import task");
        let data_dir = self.cache_dir.clone();
        let config = self.config.clone();

        Ok( spawn( &format!("bcit-{}-data-acquisition", self.config.pow_id), async move {
                run_data_acquisition( hself, config, data_dir).await
            })?.abort_handle()
        )
    }

    fn spawn_file_cleanup_task(&mut self)-> Result<AbortHandle> {
        let cache_dir = self.cache_dir.clone();
        let cleanup_interval = self.config.cleanup_interval;
        let max_age = self.config.max_age;

        Ok( spawn( &format!("bcit-{}-file-cleanup", self.config.pow_id), async move {
                run_file_cleanup( cache_dir, cleanup_interval, max_age).await
            })?.abort_handle()
        )
    }
}

impl PowerLineDataImporter for LivePowerLineImporter {
    async fn start (&mut self, hself: ActorHandle<PowerLineImportActorMsg>) -> Result<()> {
        self.initialize(hself).await?;
        Ok(())
    }

    fn terminate (&mut self) {
        if let Some(task) = &self.import_task { task.abort() }
        if let Some(task) = &self.file_cleanup_task { task.abort() }
    }
}

async fn run_data_acquisition (hself: ActorHandle<PowerLineImportActorMsg>, config: LivePowerLineImporterConfig, cache_dir: Arc<PathBuf>)->Result<()> {
    println!("running data acquisition");
    let source = Arc::new(config.source); // no need to keep gazillions of copies
    let pow_id = config.pow_id;

    // Need to start by sending the Initialize message that will have all the past data they need too.
    // Need to think more about this start point and how to update from it minimally
    let data = read_data_from_file(&source).await?;

    println!("Data read from file {:?}", data);

    // Going to transfom data to shape Initialize wants should think about fixing this so only 1 type later
    let powerlines = convert_file_data_to_powerline_struct(data);

    let powerlines_set = vec![PowerLineSet::new(powerlines)];

    hself.send_msg( Initialize(powerlines_set) ).await?;

    //--- run update loop
    loop {
        // Gonna update every 15secs to start for testing, should maybe swap this to depend on time since last finished loop
        sleep( secs(45)).await;

        let data = read_data_from_file(&source).await?;
        let mut powerlines = convert_file_data_to_powerline_struct(data);

        let mut rng = ChaChaRng::from_entropy();  // Use ChaChaRng instead of ThreadRng because async

        let num_to_drop = rng.gen_range(0..=3);
        powerlines.shuffle(&mut rng);

        // Drop the specified number of items by truncating the Vec
        let final_powerlines = powerlines.into_iter().skip(num_to_drop).collect();

        let powerlines_set = PowerLineSet::new(final_powerlines);
        // Could send an update message for each one but will just send all at once for now.
        // If there is a reason to group them in some way can change later.

        hself.send_msg(Update(powerlines_set)).await?;
    }

    Ok(())
}

async fn read_data_from_file(file_source: &str) -> Result<Vec<PowerLineDataPoint>> {
    println!("File Path to open: {}", file_source);

    let data = load_asset(file_source).expect(&format!("Didn't open file, {}", file_source));
    println!("found file");

    let mut decoder = GzDecoder::new(&data[..]);
    let mut decompressed_data = String::new();
    decoder.read_to_string(&mut decompressed_data).expect("Failed to decompress data");

    println!("Decompressed content: {:?}", decompressed_data);

    let data: Vec<PowerLineDataPoint> = serde_json::from_str(&decompressed_data)?;
    return Ok(data)
}

fn convert_file_data_to_powerline_struct(powerlines: Vec<PowerLineDataPoint>) -> Vec<PowerLine> {
    powerlines
        .into_iter()
        .enumerate()
        .map(|(index, point)| {
            let positions = point.coordinates
                .chunks(2)  // Take each pair of coordinates as (lon, lat)
                .filter_map(|chunk| {
                    if let [lat, lon] = chunk { // don't mess up the order on lat and lon here have to double check I did it right everywhere
                        Some(LatLon { lat_deg: *lat, lon_deg: *lon })
                    } else {
                        None  // Handle cases where there’s an odd number of values (this should probably be an error but this temp code anyways)
                    }
                })
                .collect();

            PowerLine {
                pow_id: index as u32,
                positions,
                time: point.time
            }
        })
        .collect()
}

async fn run_file_cleanup (cache_dir: Arc<PathBuf>, interval: Duration, max_age: Duration) {
    loop {
        let _ = remove_old_files( &cache_dir.as_path(), max_age);
        sleep(interval).await; // no need to compensate for cycle execution time
    }
}
