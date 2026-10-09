use std::path::Path;

use anyhow::{Context as _, Result};
use collections::HashMap;
use serde::Deserialize;

use crate::shell::ShellKind;

pub fn print_env() {
    let env_vars: HashMap<String, String> = std::env::vars().collect();
    let json = serde_json::to_string_pretty(&env_vars).unwrap_or_else(|err| {
        eprintln!("Error serializing environment variables: {}", err);
        std::process::exit(1);
    });
    println!("{}", json);
}

