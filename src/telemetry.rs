use url::Url;

use std::path::PathBuf;

use crate::cli::Rover;
use sputnik::{Command, Report, SputnikError};

use std::collections::HashMap;

const DEV_TELEMETRY_URL: &str = "http://localhost:8787/telemetry";
const PROD_TELEMETRY_URL: &str = "https://install.apollographql.com/telemetry";

fn get_command_from_args(mut raw_arguments: &mut serde_json::Value) -> Command {
    let mut commands = Vec::new();
    let mut arguments = HashMap::new();
    let mut should_break = true;
    loop {
        let (command_name, leftover_arguments) = get_next_command(&mut raw_arguments);
        if let Some(command_name) = command_name {
            commands.push(command_name);
            should_break = false;
        }

        if let Some(leftover_arguments) = leftover_arguments {
            if let serde_json::Value::Object(object) = leftover_arguments {
                for argument in object.iter() {
                    let (key, value) = argument;
                    arguments.insert(key.to_lowercase(), value.to_owned());
                }
            }
        }

        if should_break {
            break;
        } else {
            should_break = true;
        }
    }

    let name = commands.join(" ");
    Command { name, arguments }
}

fn get_next_command(
    raw_arguments: &mut serde_json::Value,
) -> (Option<String>, Option<serde_json::Value>) {
    let mut command_name = None;
    let mut leftover_arguments = None;

    if let Some(command_info) = raw_arguments.get("command") {
        match command_info {
            serde_json::Value::Object(object) => {
                if let Some(item) = object.clone().iter_mut().next() {
                    let (name, next) = item;
                    command_name = Some(name.to_lowercase());
                    *raw_arguments = next.to_owned();
                }
            }
            serde_json::Value::String(string) => {
                command_name = Some(string.to_lowercase());
                *raw_arguments = serde_json::Value::Null;
            }
            serde_json::Value::Null => command_name = None,
            _ => {
                command_name = Some(format!("{:?}", command_info).to_lowercase());
                *raw_arguments = serde_json::Value::Null;
            }
        }
    } else {
        leftover_arguments = Some(raw_arguments.clone());
    }

    (command_name, leftover_arguments)
}

impl Report for Rover {
    fn serialize_command(&self) -> Result<Command, SputnikError> {
        let json_args = serde_json::to_string(&self)?;
        let mut value_args = serde_json::from_str(&json_args)?;
        let serialized_command = get_command_from_args(&mut value_args);
        log::debug!("{:?}", serialized_command);
        Ok(serialized_command)
    }

    fn is_enabled(&self) -> bool {
        option_env!("APOLLO_TELEMETRY_DISABLED").is_none()
    }

    fn endpoint(&self) -> Result<Url, SputnikError> {
        if let Some(url) = option_env!("APOLLO_TELEMETRY_URL") {
            Ok(url.parse()?)
        } else if cfg!(debug_assertions) {
            Ok(DEV_TELEMETRY_URL.parse()?)
        } else {
            Ok(PROD_TELEMETRY_URL.parse()?)
        }
    }

    fn tool_name(&self) -> String {
        option_env!("CARGO_PKG_NAME")
            .unwrap_or("unknown")
            .to_string()
    }

    fn version(&self) -> String {
        option_env!("CARGO_PKG_VERSION")
            .unwrap_or("unknown")
            .to_string()
    }

    fn machine_id_config(&self) -> Result<PathBuf, SputnikError> {
        let mut path = houston::dir().map_err(|_| SputnikError::ConfigError)?;
        path.push("machine.txt");
        Ok(path)
    }
}
