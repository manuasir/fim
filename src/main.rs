// Copyright (C) 2021, Achiefs.

// To read and write directories and files
use std::fs::OpenOptions;
use std::fs;
// To get Operating system
use std::env;
// To get file system changes
use notify::{RecommendedWatcher, Watcher, RecursiveMode};
use std::sync::mpsc::channel;
// To log the program process
use log::*;
use simplelog::{WriteLogger, Config};
// To manage paths
use std::path::Path;
// To manage date and time
use std::time::{SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;
// To manage unique event identifier
use uuid::Uuid;
// To use intersperse()
use itertools::Itertools;
// To get own process ID
use std::process;

// To load hashing functions
mod hash;
// To load configuration functions
mod config;
// To load index management functions
mod index;
// To manage single event data
mod event;
use event::Event;
// To manage futures and async calls
use futures::executor::block_on;
use tokio;


fn pop(value: &str) -> &str {
    let mut chars = value.chars();
    chars.next_back();
    chars.as_str()
}

// Main function where the magic happens
#[tokio::main]
async fn main() {
    println!("System detected {}", env::consts::OS);
    println!("Reading config...");

    // Select directory where to load config.yml it depends on system
    let config_path = format!("./config/{}/config.yml", env::consts::OS);
    let path_exist = Path::new(config_path.as_str()).exists();
    let selected_path = match path_exist {
        true => config_path.as_str(),
        false => "/etc/fim/config.yml"
    };

    // Loading selected config.yml values into variables
    println!("Loaded config from: {}", selected_path);
    let config = config::read_config(selected_path);
    let version = "0.2.2";
    // Include a way to manage events destination
    let endpoint_address = String::from(config[0]["events"]["endpoint"]["address"].as_str().unwrap());
    let endpoint_user = String::from(config[0]["events"]["endpoint"]["credentials"]["user"].as_str().unwrap());
    let endpoint_pass = String::from(config[0]["events"]["endpoint"]["credentials"]["password"].as_str().unwrap());
    let monitor = &config[0]["monitor"];
    let nodename = &config[0]["nodename"];
    let log_file = &config[0]["log"]["output"]["file"].as_str().unwrap();
    let log_level = &config[0]["log"]["output"]["level"].as_str().unwrap();
    let events_file = &config[0]["log"]["events"]["file"].as_str().unwrap();

    let date = OffsetDateTime::now_utc();
    let index_name = format!("fim-{}-{}-{}", date.year(), date.month() as u8, date.day() );
    println!("{}", index_name);
    println!("Log file: {}", log_file);
    println!("Events file: {}", events_file);
    println!("Log level: {}", log_level);

    // Create folders to store logs and events based on config.yml
    fs::create_dir_all(Path::new(log_file).parent().unwrap().to_str().unwrap()).unwrap();
    fs::create_dir_all(Path::new(events_file).parent().unwrap().to_str().unwrap()).unwrap();

    // Create logger output to write generated logs.
    WriteLogger::init(
        config::get_log_level(log_level.to_string(), log_file.to_string()),
        Config::default(),
        OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(log_file)
            .expect("Unable to open log file")
    ).unwrap();

    // Iterating over monitor paths and set watcher on each folder to watch.
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new_raw(tx).unwrap();
    for m in monitor.as_vec().unwrap() {
        let path = m["path"].as_str().unwrap();
        info!("Monitoring path: {}", path);
        match m["ignore"].as_vec() {
            Some(ig) => {
                let ignore_list_vec  = ig.iter().map(|e| { e.as_str().unwrap() });
                let ignore_list : String = Itertools::intersperse(ignore_list_vec, ", ").collect();
                info!("Ignoring files with: {} inside {}", ignore_list, path);
            },
            None => {
                println!("Ignore for '{}' not set", path);
            }
        };
        watcher.watch(path, RecursiveMode::Recursive).unwrap();
    }

    // On start create index (Include check if events won't be ingested by http)
    block_on(index::create_index( index_name.clone(), endpoint_address.clone(), endpoint_user.clone(), endpoint_pass.clone()) );

    // Main loop, receive any produced event and write it into the events log.
    loop {
        match rx.recv() {
            Ok(raw_event) => {
                // Get the event path and filename
                debug!("Event registered: {:?}", raw_event);
                let event_path = Path::new(raw_event.path.as_ref().unwrap().to_str().unwrap());
                let event_parent_path = event_path.parent().unwrap().to_str().unwrap();
                let event_filename = event_path.file_name().unwrap();

                // Iterate over monitoring paths to match ignore string and ignore event or not
                let monitor_vector = monitor.as_vec().unwrap().to_vec();
                let monitor_index = monitor_vector.iter().position(|it| {
                    let path = it["path"].as_str().unwrap();
                    let value = if path.ends_with('/') || path.ends_with('\\'){ pop(path) }else{ path };
                    event_parent_path.contains(value)
                });
                let index = monitor_index.unwrap();

                if monitor_index.is_some() &&
                    match monitor_vector[index]["ignore"].as_vec() {
                        Some(igv) => ! igv.to_vec().iter().any(|ignore| event_filename.to_str().unwrap().contains(ignore.as_str().unwrap()) ),
                        None => true
                    }{

                    let current_timestamp = format!("{:?}", SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis());
                    let current_hostname = gethostname::gethostname().into_string().unwrap();
                    let yaml_labels = match monitor.as_vec().unwrap()[index]["labels"].clone().into_vec() {
                        Some(lb) => lb,
                        None => Vec::new()
                    };
                    let current_labels = yaml_labels.to_vec().iter().map(|element| String::from(element.as_str().unwrap()) ).collect();
                    let operation = raw_event.op.unwrap().clone();
                    let path = raw_event.path.unwrap().clone();

                    let event = Event {
                        id: format!("{}", Uuid::new_v4()),
                        timestamp: current_timestamp,
                        hostname: current_hostname,
                        nodename: String::from(nodename.as_str().unwrap()),
                        version: String::from(version),
                        operation: operation.clone(),
                        path: path.clone(),
                        labels: current_labels,
                        kind: event::get_kind(operation.clone()),
                        checksum: hash::get_checksum(path.to_str().unwrap().clone()),
                        pid: process::id()
                    };

                    debug!("Event received: {:?}", event);
                    event.log_event(events_file);
                    block_on(event.send( index_name.clone(), endpoint_address.clone(), endpoint_user.clone(), endpoint_pass.clone()) );
                }else{
                    debug!("Event ignored not stored in alerts");
                }
            },
            Err(e) => error!("Watch error: {:?}", e),
        }
    }
}
