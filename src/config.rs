// Copyright (C) 2021, Achiefs.

// Global constants definitions
pub const VERSION: &str = "0.4.6";
pub const NETWORK_MODE: &str = "NETWORK";
pub const FILE_MODE: &str = "FILE";
pub const BOTH_MODE: &str = "BOTH";
pub const MACHINE_ID_PATH: &str = "/etc/machine-id";
const CONFIG_MACOS_PATH: &str = "/Applications/FileMonitor.app/config.yml";
const CONFIG_LINUX_PATH: &str = "/etc/fim/config.yml";
const CONFIG_WINDOWS_PATH: &str = "C:\\Program Files\\File Integrity Monitor\\config.yml";

// To parse files in yaml format
use yaml_rust::yaml::{Yaml, YamlLoader, Array};
// To use files IO operations.
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::io::Write;
// To manage paths
use std::path::Path;
// To set log filter level
use simplelog::LevelFilter;
// To manage common functions
use crate::utils;

// ----------------------------------------------------------------------------

#[derive(Clone)]
pub struct Config {
    pub version: String,
    pub path: String,
    pub events_destination: String,
    pub events_max_file_checksum: usize,
    pub endpoint_address: String,
    pub endpoint_user: String,
    pub endpoint_pass: String,
    pub events_file: String,
    pub monitor: Array,
    pub audit: Array,
    pub node: String,
    pub log_file: String,
    pub log_level: String,
    pub system: String,
    pub insecure: bool
}

impl Config {

    pub fn clone(&self) -> Self {
        Config {
            version: self.version.clone(),
            path: self.path.clone(),
            events_destination: self.events_destination.clone(),
            events_max_file_checksum: self.events_max_file_checksum,
            endpoint_address: self.endpoint_address.clone(),
            endpoint_user: self.endpoint_user.clone(),
            endpoint_pass: self.endpoint_pass.clone(),
            events_file: self.events_file.clone(),
            monitor: self.monitor.clone(),
            audit: self.audit.clone(),
            node: self.node.clone(),
            log_file: self.log_file.clone(),
            log_level: self.log_level.clone(),
            system: self.system.clone(),
            insecure: self.insecure
        }
    }

    pub fn new(system: &str, config_path: Option<&str>) -> Self {
        println!("System detected '{}'", system);
        let cfg = match config_path {
            Some(path) => String::from(path),
            None => get_config_path(system)
        };
        println!("Loaded config from: '{}'", cfg);
        let yaml = read_config(cfg.clone());

        // Manage null value on events->destination value
        let events_destination = match yaml[0]["events"]["destination"].as_str() {
            Some(value) => String::from(value),
            None => {
                println!("[WARN] events->destination not found in config.yml, using 'file'.");
                String::from("file")
            }
        };

        // Manage null value on events->file value
        let events_file = match yaml[0]["events"]["file"].as_str() {
            Some(value) => String::from(value),
            None => {
                if events_destination != *"network" {
                    println!("[ERROR] events->file not found in config.yml.");
                    panic!("events->file not found in config.yml.");
                }else{
                    String::from("Not_used")
                }
            }
        };

        // Manage null value on events->max_file_checksum value
        let events_max_file_checksum = match yaml[0]["events"]["max_file_checksum"].as_i64() {
            Some(value) => usize::try_from(value).unwrap(),
            None => 64
        };

        // Manage null value on events->endpoint->insecure value
        let insecure = match yaml[0]["events"]["endpoint"]["insecure"].as_bool() {
            Some(value) => value,
            None => {
                if events_destination != *"file" {
                    println!("[WARN] events->endpoint->insecure not found in config.yml, using 'false'.");
                    false
                }else{ false }
            }
        };

        // Manage null value on events->endpoint->address value
        let endpoint_address = match yaml[0]["events"]["endpoint"]["address"].as_str() {
            Some(value) => String::from(value),
            None => {
                if events_destination != *"file" {
                    println!("[ERROR] events->endpoint->address not found in config.yml.");
                    panic!("events->endpoint->address not found in config.yml.");
                }else{
                    String::from("Not_used")
                }
            }
        };

        // Manage null value on events->endpoint->credentials->user value
        let endpoint_user = match yaml[0]["events"]["endpoint"]["credentials"]["user"].as_str() {
            Some(value) => String::from(value),
            None => {
                if events_destination != *"file" {
                    println!("[ERROR] events->endpoint->credentials->user not found in config.yml.");
                    panic!("events->endpoint->credentials->user not found in config.yml.");
                }else{
                    String::from("Not_used")
                }
            }
        };

        // Manage null value on events->endpoint->credentials->password value
        let endpoint_pass = match yaml[0]["events"]["endpoint"]["credentials"]["password"].as_str() {
            Some(value) => String::from(value),
            None => {
                if events_destination != *"file" {
                    println!("[ERROR] events->endpoint->credentials->password not found in config.yml.");
                    panic!("events->endpoint->credentials->password not found in config.yml.");
                }else{
                    String::from("Not_used")
                }
            }
        };

        // Manage null value on monitor value
        let monitor = match yaml[0]["monitor"].as_vec() {
            Some(value) => value.to_vec(),
            None => Vec::new()
        };

        // Manage null value on audit value
        let audit = match yaml[0]["audit"].as_vec() {
            Some(value) => {
                if utils::get_os() != "linux"{
                    panic!("Audit only supported in Linux systems.");
                }
                value.to_vec()
            },
            None => {
                if monitor.is_empty() {
                    panic!("Neither monitor or audit section found in config.yml.");
                };
                Vec::new()
            }
        };

        // Manage null value on node value
        let node = match yaml[0]["node"].as_str() {
            Some(value) => String::from(value),
            None => {
                match system {
                    "linux" => match utils::get_machine_id().is_empty() {
                        true => utils::get_hostname(),
                        false => utils::get_machine_id()
                    },
                    "macos" => match utils::get_machine_id().is_empty(){
                        true => utils::get_hostname(),
                        false => utils::get_machine_id()
                    }
                    _ => {
                        println!("[WARN] node not found in config.yml, using hostname.");
                        utils::get_hostname()
                    }
                }
            }
        };

        // Manage null value on log->file value
        let log_file = match yaml[0]["log"]["file"].as_str() {
            Some(value) => String::from(value),
            None => {
                println!("[ERROR] log->file not found in config.yml.");
                panic!("log->file not found in config.yml.");
            }
        };

        // Manage null value on log->level value
        let log_level = match yaml[0]["log"]["level"].as_str() {
            Some(value) => String::from(value),
            None => {
                println!("[WARN] log->level not found in config.yml, using 'info'.");
                String::from("info")
            }
        };

        Config {
            version: String::from(VERSION),
            path: cfg,
            events_destination,
            events_max_file_checksum,
            endpoint_address,
            endpoint_user,
            endpoint_pass,
            events_file,
            monitor,
            audit,
            node,
            log_file,
            log_level,
            system: String::from(system),
            insecure
        }
    }

    // ------------------------------------------------------------------------

    // To process log level set on config file
    pub fn get_level_filter(&self) -> LevelFilter {
        let mut log = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(self.log_file.clone())
            .expect("(get_level_filter) Unable to open events log file.");

        match self.log_level.as_str() {
            "debug" | "Debug" | "DEBUG" | "D" | "d" => LevelFilter::Debug,
            "info" | "Info" | "INFO" | "I" | "i" => LevelFilter::Info,
            "error" | "Error" | "ERROR" | "E" | "e" => LevelFilter::Error,
            "warning" | "Warning" | "WARNING" | "W" | "w" | "warn" | "Warn" | "WARN" => LevelFilter::Warn,
            _ => {
                let msg = String::from("[ERROR] invalid log level from 'config.yml', using Info level.");
                println!("{}", msg);
                writeln!(log, "{}", msg).expect("[ERROR] cannot write in log file.");
                LevelFilter::Info
            }
        }
    }

    // ------------------------------------------------------------------------

    pub fn get_events_destination(&self) -> String {
        match self.events_destination.clone().as_str() {
            "both" => String::from(BOTH_MODE),
            "network" => String::from(NETWORK_MODE),
            // Default option is to log into file
            _ => String::from(FILE_MODE)
        }
    }

    // ------------------------------------------------------------------------

    pub fn get_index(&self, raw_path: &str, cwd: &str, array: Array) -> usize {
        // Iterate over monitoring paths to match ignore string and ignore event or not
        match array.iter().position(|it| {
            if !cwd.is_empty() && (raw_path.starts_with("./") || raw_path == "." || !raw_path.contains('/')) {
                utils::match_path(cwd, it["path"].as_str().unwrap())
            }else{
                utils::match_path(raw_path, it["path"].as_str().unwrap())
            }
        }){
            Some(pos) => pos,
            None => usize::MAX
        }
    }

    // ------------------------------------------------------------------------

    pub fn get_labels(&self, index: usize, array: Array) -> Vec<String> {
        match array[index]["labels"].clone().into_vec() {
            Some(labels) => labels,
            None => Vec::new()
        }.to_vec().iter().map(|element| String::from(element.as_str().unwrap()) ).collect()
    }

    // ------------------------------------------------------------------------

    pub fn match_ignore(&self, index: usize, filename: &str, array: Array) -> bool {
        match array[index]["ignore"].as_vec() {
            Some(igv) => igv.to_vec().iter().any(|ignore| filename.contains(ignore.as_str().unwrap()) ),
            None => false
        }
    }

    // ------------------------------------------------------------------------

    // Returns if a given path and filename is in the configuration paths
    pub fn path_in(&self, raw_path: &str, cwd: &str, vector: Vec<Yaml>) -> bool {
        // Iterate over monitoring paths to match ignore string and ignore event or not
        match vector.iter().any(|it| {
            if raw_path.starts_with("./") || raw_path == "." || !raw_path.contains('/') {
                utils::match_path(cwd, it["path"].as_str().unwrap())
            }else{
                utils::match_path(raw_path, it["path"].as_str().unwrap())
            }
        }){
            true => true,
            false => false
        }
    }

}



// ----------------------------------------------------------------------------

// To read the Yaml configuration file
pub fn read_config(path: String) -> Vec<Yaml> {
    let mut file: File = File::open(path.clone())
        .unwrap_or_else(|_| panic!("(read_config): Unable to open file '{}'", path));
    let mut contents: String = String::new();

    file.read_to_string(&mut contents)
        .expect("Unable to read file");
    YamlLoader::load_from_str(&contents).unwrap()
}

// ----------------------------------------------------------------------------

pub fn get_config_path(system: &str) -> String {
    // Select directory where to load config.yml it depends on system
    let current_dir: String = utils::get_current_dir();
    if system == "windows" {
        let default_path: String = format!("{}\\config\\{}\\config.yml", current_dir, system);
        let relative_path: String = format!("{}\\..\\..\\config\\{}\\config.yml", current_dir, system);
        if Path::new(default_path.as_str()).exists() {
            default_path
        }else if Path::new(&format!("{}\\config.yml", current_dir)).exists() {
            format!("{}\\config.yml", current_dir)
        }else if Path::new(relative_path.as_str()).exists() {
            relative_path
        }else{
            String::from(CONFIG_WINDOWS_PATH)
        }
    }else{
        let default_path: String = format!("{}/config/{}/config.yml", current_dir, system);
        let relative_path: String = format!("{}/../../config/{}/config.yml", current_dir, system);
        if Path::new(default_path.as_str()).exists() {
            default_path
        }else if Path::new(&format!("{}/config.yml", current_dir)).exists() {
            format!("{}/config.yml", current_dir)
        }else if Path::new(relative_path.as_str()).exists() {
            relative_path
        }else if system == "macos" {
            String::from(CONFIG_MACOS_PATH)
        }else{
            String::from(CONFIG_LINUX_PATH)
        } 
    }
}

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------

    pub fn create_test_config(filter: &str, events_destination: &str) -> Config {
        Config {
            version: String::from(VERSION),
            path: String::from("test"),
            events_destination: String::from(events_destination),
            events_max_file_checksum: 64,
            endpoint_address: String::from("test"),
            endpoint_user: String::from("test"),
            endpoint_pass: String::from("test"),
            events_file: String::from("test"),
            monitor: Array::new(),
            audit: Array::new(),
            node: String::from("test"),
            log_file: String::from("./test.log"),
            log_level: String::from(filter),
            system: String::from("test"),
            insecure: true
        }
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_clone() {
        let config = create_test_config("info", "");
        let cloned = config.clone();
        assert_eq!(config.version, cloned.version);
        assert_eq!(config.path, cloned.path);
        assert_eq!(config.events_destination, cloned.events_destination);
        assert_eq!(config.events_max_file_checksum, cloned.events_max_file_checksum);
        assert_eq!(config.endpoint_address, cloned.endpoint_address);
        assert_eq!(config.endpoint_user, cloned.endpoint_user);
        assert_eq!(config.endpoint_pass, cloned.endpoint_pass);
        assert_eq!(config.events_file, cloned.events_file);
        assert_eq!(config.monitor, cloned.monitor);
        assert_eq!(config.audit, cloned.audit);
        assert_eq!(config.node, cloned.node);
        assert_eq!(config.log_file, cloned.log_file);
        assert_eq!(config.log_level, cloned.log_level);
        assert_eq!(config.system, cloned.system);
        assert_eq!(config.insecure, cloned.insecure);
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_new_config_windows() {
        let config = Config::new("windows", None);
        assert_eq!(config.version, String::from(VERSION));
        assert_eq!(config.events_destination, String::from("file"));
        assert_eq!(config.endpoint_address, String::from("Not_used"));
        assert_eq!(config.endpoint_user, String::from("Not_used"));
        assert_eq!(config.endpoint_pass, String::from("Not_used"));
        assert_eq!(config.events_file, String::from("C:\\ProgramData\\fim\\events.json"));
        // monitor
        // audit
        assert_eq!(config.node, String::from("FIM"));
        assert_eq!(config.log_file, String::from("C:\\ProgramData\\fim\\fim.log"));
        assert_eq!(config.log_level, String::from("info"));
        assert_eq!(config.system, String::from("windows"));
        assert_eq!(config.insecure, false);
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_new_config_windows_events_destination() {
        let config = Config::new("windows", Some("test/unit/config/windows/events_destination_none.yml"));
        assert_eq!(config.events_destination, String::from("file"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    #[should_panic]
    fn test_new_config_windows_events_file() {
        Config::new("windows", Some("test/unit/config/windows/events_file_none.yml"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_new_config_windows_events_destination_network() {
        let config = Config::new("windows", Some("test/unit/config/windows/events_destination_network.yml"));
        assert_eq!(config.events_file, String::from("Not_used"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_new_config_windows_events_max_file_checksum() {
        let config = Config::new("windows", Some("test/unit/config/windows/events_max_file_checksum.yml"));
        assert_eq!(config.events_max_file_checksum, 128);
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_new_config_windows_events_endpoint_insecure() {
        let config = Config::new("windows", Some("test/unit/config/windows/events_endpoint_insecure.yml"));
        assert_eq!(config.insecure, true);
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_new_config_windows_events_endpoint_insecure_none() {
        let config = Config::new("windows", Some("test/unit/config/windows/events_endpoint_insecure_none.yml"));
        assert_eq!(config.insecure, false);
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_new_config_windows_events_destination_network_address() {
        let config = Config::new("windows", Some("test/unit/config/windows/events_destination_network_address.yml"));
        assert_eq!(config.endpoint_address, "0.0.0.0");
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    #[should_panic]
    fn test_new_config_windows_events_destination_network_address_none() {
        Config::new("windows", Some("test/unit/config/windows/events_destination_network_address_none.yml"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_new_config_windows_events_credentials_user() {
        let config = Config::new("windows", Some("test/unit/config/windows/events_credentials_user.yml"));
        assert_eq!(config.endpoint_user, "test");
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    #[should_panic]
    fn test_new_config_windows_events_credentials_user_none() {
        Config::new("windows", Some("test/unit/config/windows/events_credentials_user_none.yml"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_new_config_windows_events_credentials_password() {
        let config = Config::new("windows", Some("test/unit/config/windows/events_credentials_password.yml"));
        assert_eq!(config.endpoint_pass, "test");
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    #[should_panic]
    fn test_new_config_windows_events_credentials_password_none() {
        Config::new("windows", Some("test/unit/config/windows/events_credentials_password_none.yml"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    #[should_panic]
    fn test_new_config_windows_monitor_none() {
        Config::new("windows", Some("test/unit/config/windows/monitor_none.yml"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_new_config_windows_node_none() {
        let config = Config::new("windows", Some("test/unit/config/windows/node_none.yml"));
        assert_eq!(config.node, utils::get_hostname());
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    #[should_panic]
    fn test_new_config_windows_log_file_none() {
        Config::new("windows", Some("test/unit/config/windows/log_file_none.yml"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_new_config_windows_log_level_none() {
        let config = Config::new("windows", Some("test/unit/config/windows/log_level_none.yml"));
        assert_eq!(config.log_level, "info");
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    fn test_new_config_linux_events_destination() {
        let config = Config::new("linux", Some("test/unit/config/linux/events_destination_none.yml"));
        assert_eq!(config.events_destination, String::from("file"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    #[should_panic]
    fn test_new_config_linux_events_file() {
        Config::new("linux", Some("test/unit/config/linux/events_file_none.yml"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    fn test_new_config_linux_events_destination_network() {
        let config = Config::new("linux", Some("test/unit/config/linux/events_destination_network.yml"));
        assert_eq!(config.events_file, String::from("Not_used"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    fn test_new_config_linux_events_max_file_checksum() {
        let config = Config::new("linux", Some("test/unit/config/linux/events_max_file_checksum.yml"));
        assert_eq!(config.events_max_file_checksum, 128);
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    fn test_new_config_linux_events_endpoint_insecure() {
        let config = Config::new("linux", Some("test/unit/config/linux/events_endpoint_insecure.yml"));
        assert_eq!(config.insecure, true);
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    fn test_new_config_linux_events_endpoint_insecure_none() {
        let config = Config::new("linux", Some("test/unit/config/linux/events_endpoint_insecure_none.yml"));
        assert_eq!(config.insecure, false);
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    fn test_new_config_linux_events_destination_network_address() {
        let config = Config::new("linux", Some("test/unit/config/linux/events_destination_network_address.yml"));
        assert_eq!(config.endpoint_address, "0.0.0.0");
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    #[should_panic]
    fn test_new_config_linux_events_destination_network_address_none() {
        Config::new("linux", Some("test/unit/config/linux/events_destination_network_address_none.yml"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    fn test_new_config_linux_events_credentials_user() {
        let config = Config::new("linux", Some("test/unit/config/linux/events_credentials_user.yml"));
        assert_eq!(config.endpoint_user, "test");
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    #[should_panic]
    fn test_new_config_linux_events_credentials_user_none() {
        Config::new("linux", Some("test/unit/config/linux/events_credentials_user_none.yml"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    fn test_new_config_linux_events_credentials_password() {
        let config = Config::new("linux", Some("test/unit/config/linux/events_credentials_password.yml"));
        assert_eq!(config.endpoint_pass, "test");
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    #[should_panic]
    fn test_new_config_linux_events_credentials_password_none() {
        Config::new("linux", Some("test/unit/config/linux/events_credentials_password_none.yml"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    fn test_new_config_linux_monitor_none() {
        let config = Config::new("linux", Some("test/unit/config/linux/monitor_none.yml"));
        assert_eq!(config.monitor, Vec::new());
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    fn test_new_config_linux_audit_none() {
        let config = Config::new("linux", Some("test/unit/config/linux/audit_none.yml"));
        assert_eq!(config.audit, Vec::new());
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    #[should_panic]
    fn test_new_config_linux_audit_and_monitor_none() {
        Config::new("linux", Some("test/unit/config/linux/audit_and_monitor_none.yml"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    fn test_new_config_linux_node_none() {
        let config = Config::new("linux", Some("test/unit/config/linux/node_none.yml"));
        let machine_id = utils::get_machine_id();
        match machine_id.is_empty(){
            true => assert_eq!(config.node, utils::get_hostname()),
            false => assert_eq!(config.node, machine_id)
        }
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    #[should_panic]
    fn test_new_config_linux_log_file_none() {
        Config::new("linux", Some("test/unit/config/linux/log_file_none.yml"));
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    fn test_new_config_linux_log_level_none() {
        let config = Config::new("linux", Some("test/unit/config/linux/log_level_none.yml"));
        assert_eq!(config.log_level, "info");
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_new_config_linux() {
        if utils::get_os() == "linux" {
            let config = Config::new("linux", None);
            assert_eq!(config.version, String::from(VERSION));
            assert_eq!(config.events_destination, String::from("file"));
            assert_eq!(config.endpoint_address, String::from("Not_used"));
            assert_eq!(config.endpoint_user, String::from("Not_used"));
            assert_eq!(config.endpoint_pass, String::from("Not_used"));
            assert_eq!(config.events_file, String::from("/var/lib/fim/events.json"));
            // monitor
            // audit
            assert_eq!(config.node, String::from("FIM"));
            assert_eq!(config.log_file, String::from("/var/log/fim/fim.log"));
            assert_eq!(config.log_level, String::from("info"));
            assert_eq!(config.system, String::from("linux"));
            assert_eq!(config.insecure, false);
        }
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_new_config_macos() {
        let config = Config::new("macos", None);
        assert_eq!(config.version, String::from(VERSION));
        assert_eq!(config.events_destination, String::from("file"));
        assert_eq!(config.endpoint_address, String::from("Not_used"));
        assert_eq!(config.endpoint_user, String::from("Not_used"));
        assert_eq!(config.endpoint_pass, String::from("Not_used"));
        assert_eq!(config.events_file, String::from("/var/lib/fim/events.json"));
        // monitor
        // audit
        assert_eq!(config.node, String::from("FIM"));
        assert_eq!(config.log_file, String::from("/var/log/fim/fim.log"));
        assert_eq!(config.log_level, String::from("info"));
        assert_eq!(config.system, String::from("macos"));
        assert_eq!(config.insecure, false);
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_get_level_filter_info() {
        let filter = LevelFilter::Info;
        assert_eq!(create_test_config("info", "").get_level_filter(), filter);
        assert_eq!(create_test_config("Info", "").get_level_filter(), filter);
        assert_eq!(create_test_config("INFO", "").get_level_filter(), filter);
        assert_eq!(create_test_config("I", "").get_level_filter(), filter);
        assert_eq!(create_test_config("i", "").get_level_filter(), filter);
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_get_level_filter_debug() {
        let filter = LevelFilter::Debug;
        assert_eq!(create_test_config("debug", "").get_level_filter(), filter);
        assert_eq!(create_test_config("Debug", "").get_level_filter(), filter);
        assert_eq!(create_test_config("DEBUG", "").get_level_filter(), filter);
        assert_eq!(create_test_config("D", "").get_level_filter(), filter);
        assert_eq!(create_test_config("d", "").get_level_filter(), filter);
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_get_level_filter_error() {
        let filter = LevelFilter::Error;
        assert_eq!(create_test_config("error", "").get_level_filter(), filter);
        assert_eq!(create_test_config("Error", "").get_level_filter(), filter);
        assert_eq!(create_test_config("ERROR", "").get_level_filter(), filter);
        assert_eq!(create_test_config("E", "").get_level_filter(), filter);
        assert_eq!(create_test_config("e", "").get_level_filter(), filter);
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_get_level_filter_warning() {
        let filter = LevelFilter::Warn;
        assert_eq!(create_test_config("warning", "").get_level_filter(), filter);
        assert_eq!(create_test_config("Warning", "").get_level_filter(), filter);
        assert_eq!(create_test_config("WARNING", "").get_level_filter(), filter);
        assert_eq!(create_test_config("W", "").get_level_filter(), filter);
        assert_eq!(create_test_config("w", "").get_level_filter(), filter);
        assert_eq!(create_test_config("warn", "").get_level_filter(), filter);
        assert_eq!(create_test_config("Warn", "").get_level_filter(), filter);
        assert_eq!(create_test_config("WARN", "").get_level_filter(), filter);
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_get_level_filter_bad() {
        let filter = LevelFilter::Info;
        assert_eq!(create_test_config("bad", "").get_level_filter(), filter);
        assert_eq!(create_test_config("BAD", "").get_level_filter(), filter);
        assert_eq!(create_test_config("B", "").get_level_filter(), filter);
        assert_eq!(create_test_config("b", "").get_level_filter(), filter);
        assert_eq!(create_test_config("test", "").get_level_filter(), filter);
        assert_eq!(create_test_config("", "").get_level_filter(), filter);
        assert_eq!(create_test_config("_", "").get_level_filter(), filter);
        assert_eq!(create_test_config("?", "").get_level_filter(), filter);
        assert_eq!(create_test_config("=", "").get_level_filter(), filter);
        assert_eq!(create_test_config("/", "").get_level_filter(), filter);
        assert_eq!(create_test_config(".", "").get_level_filter(), filter);
        assert_eq!(create_test_config(":", "").get_level_filter(), filter);
        assert_eq!(create_test_config(";", "").get_level_filter(), filter);
        assert_eq!(create_test_config("!", "").get_level_filter(), filter);
        assert_eq!(create_test_config("''", "").get_level_filter(), filter);
        assert_eq!(create_test_config("[]", "").get_level_filter(), filter);
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_get_events_destination() {
        assert_eq!(create_test_config("info", "both").get_events_destination(), String::from(BOTH_MODE));
        assert_eq!(create_test_config("info", "network").get_events_destination(), String::from(NETWORK_MODE));
        assert_eq!(create_test_config("info", "file").get_events_destination(), String::from(FILE_MODE));
        assert_eq!(create_test_config("info", "").get_events_destination(), String::from(FILE_MODE));
        assert_eq!(create_test_config("info", "?").get_events_destination(), String::from(FILE_MODE));
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_read_config_unix() {
        let yaml = read_config(String::from("config/linux/config.yml"));

        assert_eq!(yaml[0]["node"].as_str().unwrap(), "FIM");
        assert_eq!(yaml[0]["events"]["destination"].as_str().unwrap(), "file");
        assert_eq!(yaml[0]["events"]["file"].as_str().unwrap(), "/var/lib/fim/events.json");

        assert_eq!(yaml[0]["monitor"][0]["path"].as_str().unwrap(), "/bin/");
        assert_eq!(yaml[0]["monitor"][1]["path"].as_str().unwrap(), "/usr/bin/");
        assert_eq!(yaml[0]["monitor"][1]["labels"][0].as_str().unwrap(), "usr/bin");
        assert_eq!(yaml[0]["monitor"][1]["labels"][1].as_str().unwrap(), "linux");
        assert_eq!(yaml[0]["monitor"][2]["path"].as_str().unwrap(), "/etc");
        assert_eq!(yaml[0]["monitor"][2]["labels"][0].as_str().unwrap(), "etc");
        assert_eq!(yaml[0]["monitor"][2]["labels"][1].as_str().unwrap(), "linux");

        assert_eq!(yaml[0]["log"]["file"].as_str().unwrap(), "/var/log/fim/fim.log");
        assert_eq!(yaml[0]["log"]["level"].as_str().unwrap(), "info");
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_read_config_windows() {
        let yaml = read_config(String::from("config/windows/config.yml"));

        assert_eq!(yaml[0]["node"].as_str().unwrap(), "FIM");
        assert_eq!(yaml[0]["events"]["destination"].as_str().unwrap(), "file");
        assert_eq!(yaml[0]["events"]["file"].as_str().unwrap(), "C:\\ProgramData\\fim\\events.json");

        assert_eq!(yaml[0]["monitor"][0]["path"].as_str().unwrap(), "C:\\Program Files\\");
        assert_eq!(yaml[0]["monitor"][0]["labels"][0].as_str().unwrap(), "Program Files");
        assert_eq!(yaml[0]["monitor"][0]["labels"][1].as_str().unwrap(), "windows");
        assert_eq!(yaml[0]["monitor"][1]["path"].as_str().unwrap(), "C:\\Users\\");
        assert_eq!(yaml[0]["monitor"][1]["labels"][0].as_str().unwrap(), "Users");
        assert_eq!(yaml[0]["monitor"][1]["labels"][1].as_str().unwrap(), "windows");

        assert_eq!(yaml[0]["log"]["file"].as_str().unwrap(), "C:\\ProgramData\\fim\\fim.log");
        assert_eq!(yaml[0]["log"]["level"].as_str().unwrap(), "info");
    }

    // ------------------------------------------------------------------------

    #[test]
    #[should_panic(expected = "NotFound")]
    fn test_read_config_panic() {
        read_config(String::from("NotFound"));
    }

    // ------------------------------------------------------------------------

    #[test]
    #[should_panic(expected = "ScanError")]
    fn test_read_config_panic_not_config() {
        read_config(String::from("README.md"));
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_get_config_path_unix() {
        let current_dir = utils::get_current_dir();
        let default_path_linux = format!("{}/config/linux/config.yml", current_dir);
        let default_path_macos = format!("{}/config/macos/config.yml", current_dir);
        assert_eq!(get_config_path("linux"), default_path_linux);
        assert_eq!(get_config_path("macos"), default_path_macos);
    }

    // ------------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_get_config_path_windows() {
        let current_dir = utils::get_current_dir();
        let default_path_windows = format!("{}\\config\\windows\\config.yml", current_dir);
        assert_eq!(get_config_path("windows"), default_path_windows);
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_path_in() {
        let config = Config::new(&utils::get_os(), None);
        if utils::get_os() == "linux" {
            assert!(config.path_in("/bin/", "", config.monitor.clone()));
            assert!(config.path_in("/bin", "", config.monitor.clone()));
            assert!(config.path_in("/bin/test", "", config.monitor.clone()));
            assert!(!config.path_in("/test", "", config.monitor.clone()));
            assert!(config.path_in("/tmp", "", config.audit.clone()));
            assert!(config.path_in("/tmp/", "", config.audit.clone()));
            assert!(config.path_in("./", "/tmp", config.audit.clone()));
            assert!(config.path_in("./", "/tmp/", config.audit.clone()));
            assert!(!config.path_in("./", "/test", config.audit.clone()));
            assert!(config.path_in("./", "/tmp/test", config.audit.clone()));
        }
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_get_index() {
        let config = Config::new(&utils::get_os(), None);
        if utils::get_os() == "linux" {
            assert_eq!(config.get_index("/bin/", "", config.monitor.clone()), 0);
            assert_eq!(config.get_index("./", "/bin", config.monitor.clone()), 0);
            assert_eq!(config.get_index("/usr/bin/", "", config.monitor.clone()), 1);
            assert_eq!(config.get_index("/etc", "", config.monitor.clone()), 2);
            assert_eq!(config.get_index("/test", "", config.monitor.clone()), usize::MAX);
            assert_eq!(config.get_index("./", "/test", config.monitor.clone()), usize::MAX);
            assert_eq!(config.get_index("/tmp", "", config.audit.clone()), 0);
            assert_eq!(config.get_index("/test", "", config.audit.clone()), usize::MAX);
            assert_eq!(config.get_index("./", "/tmp", config.audit.clone()), 0);
            assert_eq!(config.get_index("./", "/test", config.audit.clone()), usize::MAX);
        }
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_get_labels() {
        let config = Config::new(&utils::get_os(), None);
        if utils::get_os() == "windows" {
            let labels = config.get_labels(0, config.monitor.clone());
            assert_eq!(labels[0], "Program Files");
            assert_eq!(labels[1], "windows");
        }else if utils::get_os() == "macos"{
            let labels = config.get_labels(2, config.monitor.clone());
            assert_eq!(labels[0], "usr/bin");
            assert_eq!(labels[1], "macos");
        }else{
            let labels = config.get_labels(1, config.monitor.clone());
            assert_eq!(labels[0], "usr/bin");
            assert_eq!(labels[1], "linux");

            let labels = config.get_labels(0, config.audit.clone());
            assert_eq!(labels[0], "tmp");
            assert_eq!(labels[1], "linux");
        }
    }

    // ------------------------------------------------------------------------

    #[test]
    fn test_match_ignore() {
        let config = Config::new(&utils::get_os(), None);
        if utils::get_os() == "linux" {
            assert!(config.match_ignore(0, "file.swp", config.audit.clone()));
            assert!(!config.match_ignore(0, "file.txt", config.audit.clone()));
        }
    }

}
