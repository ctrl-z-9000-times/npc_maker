//! API for making new environments.

use super::{EnvironmentSpec, Error, Mode, Request};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::os::fd::AsRawFd;
use std::path::Path;

/// Read the command line arguments for an environment program.
///
/// Environment implementations *must* call this function for initialization purposes.
///
/// Returns a tuple of (environment-specification, graphics-mode, settings-dict)
pub fn get_args() -> (EnvironmentSpec, Mode, HashMap<String, String>) {
    init();
    // Read the command line arguments.
    let mut arg_iter = std::env::args();
    let _program = arg_iter.next();
    let spec_file = arg_iter.next();
    let mode = arg_iter.next();
    let mut settings: Vec<String> = arg_iter.collect();
    // Read the environment specification file.
    let Some(spec_file) = spec_file else {
        panic!("Argument Error: missing environment specification")
    };
    let spec_file = Path::new(&spec_file)
        .canonicalize()
        .unwrap_or_else(|err| panic!("File Error: {err}: {spec_file:?}"));
    let spec_data =
        std::fs::read_to_string(&spec_file).unwrap_or_else(|err| panic!("File Error: {err}: {spec_file:?}"));
    let mut env_spec: EnvironmentSpec =
        serde_json::from_str(&spec_data).unwrap_or_else(|err| panic!("JSON Decode Error: {err}: {spec_file:?}"));
    env_spec.spec = spec_file;
    // Read the graphics mode.
    let mode = if let Some(mode) = mode {
        mode.parse().unwrap_or_else(|err| panic!("Argument Error: {err}"))
    } else {
        Mode::default()
    };
    // Assemble the settings dictionary.
    let mut defaults: HashMap<String, _> = env_spec
        .settings
        .iter()
        .map(|item| (item.name().to_string(), item.default()))
        .collect();
    let mut settings = settings.chunks_exact_mut(2);
    for chunk in &mut settings {
        let item = std::mem::take(&mut chunk[0]);
        let value = std::mem::take(&mut chunk[1]);
        if !defaults.contains_key(&item) {
            panic!("Argument Error: unexpected parameter \"{item}\"")
        }
        defaults.insert(item, value);
    }
    if !settings.into_remainder().is_empty() {
        panic!("Argument Error: odd number of settings, expected key-value pairs");
    }
    //
    (env_spec, mode, defaults)
}

fn init() {
    #[cfg(target_family = "unix")]
    {
        change_blocking_fd(io::stdin().as_raw_fd(), false);
    }
    #[cfg(target_family = "windows")]
    {
        todo!()
    }
}

#[cfg(target_family = "unix")]
fn change_blocking_fd(fd: std::os::unix::io::RawFd, blocking: bool) {
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags < 0 {
            panic!("libc file control error");
        }
        let error = libc::fcntl(
            fd,
            libc::F_SETFL,
            if blocking {
                flags & !libc::O_NONBLOCK
            } else {
                flags | libc::O_NONBLOCK
            },
        );
        if error < 0 {
            panic!("libc file control error");
        }
    }
}

/// Check for messages from the main NPC Maker program.
///
/// Callers *must* call the `get_args()` function before this, for initialization purposes.
///
/// This function is non-blocking and returns `None` if there are no new
/// messages. This decodes the JSON messages and returns `Request` objects.
pub fn poll() -> Result<Option<Request>, Error> {
    // Read a line from stdin, non blocking.
    let mut line = String::new();
    if let Err(error) = io::stdin().lock().read_line(&mut line) {
        if error.kind() == io::ErrorKind::WouldBlock {
            io::stdout().flush()?;
            return Ok(None);
        } else {
            return Err(error.into());
        }
    }
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }
    // Parse the message.
    match serde_json::from_str(line) {
        Err(error) => {
            if false {
                // Ignore invalid data (cat on keyboard).
                eprintln!("JSON decode error {error}");
                Ok(None)
            } else {
                // Propagate errors to the caller.
                Err(error.into())
            }
        }
        Ok(message) => Ok(Some(message)),
    }
}

/// Acknowledge that the given message has been successfully acted upon.
pub fn ack(message: &Request) {
    // Birth messages don't need to be acknowledged.
    if let Request::Birth { .. } = message {
        return;
    }
    let mut stdout = io::stdout().lock();
    write!(stdout, "{{\"Ack\":").unwrap();
    serde_json::to_writer(&mut stdout, message).unwrap();
    writeln!(stdout, "}}").unwrap();
}

/// Request a new individual from the evolutionary algorithm.
///
/// Argument population is optional if the environment contains exactly one population.
pub fn new(population: Option<&str>) {
    println!(r#"{{"New":"{}"}}"#, population.unwrap_or(""));
}

/// Request to mate two specific individuals together to produce a child individual.
pub fn mate(parent1: &str, parent2: &str) {
    println!(r#"{{"Mate":["{parent1}","{parent2}"]}}"#);
}

/// Report an individual's score or reproductive fitness to the evolutionary algorithm.
///
/// This should be called *before* calling [death] on the individual.
///
/// Argument individual is optional if the environment contains exactly one individual.
pub fn score(individual: Option<&str>, value: &str) {
    println!(r#"{{"Score":"{value}","name":"{}"}}"#, individual.unwrap_or(""));
}

/// Report extra information about an individual.
///
/// Argument info is a mapping of string key-value pairs.
///
/// Argument individual is optional if the environment contains exactly one individual.
pub fn info(individual: Option<&str>, info: &HashMap<String, String>) {
    let mut json = String::new();
    for (key, value) in info {
        json.push('"');
        json.push_str(key);
        json.push('"');
        json.push(':');
        json.push('"');
        json.push_str(value);
        json.push('"');
        json.push(',');
    }
    json.pop(); // Remove trailing comma.
    println!(r#"{{"Info":{{{json}}},"name":"{}"}}"#, individual.unwrap_or(""));
}

/// Notify the evolutionary algorithm that the given individual has died.
///
/// The individual's score or reproductive fitness should be reported
/// using the [score()] function *before* calling this method.
///
/// Argument individual is optional if the environment contains exactly one individual.
pub fn death(individual: Option<&str>) {
    println!(r#"{{"Death":"{}"}}"#, individual.unwrap_or(""));
}
