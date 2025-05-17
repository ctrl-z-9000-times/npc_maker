//! Environment Interface, for making and using environments.
//!
//! Each environment runs in its own computer process and uses stdin & stdout to
//! communicate with the evolutionary algorithm and the main NPC Maker program.
//! Environments should use stderr to report any unformatted or diagnostic messages
//! (see [eprintln!()]).

mod messages;
mod specification;

pub use messages::{Request, Response};
pub use specification::{EnvironmentSpec, InterfaceSpec, PopulationSpec, SettingsSpec};

/*

use crate::Error as JsonIoError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::str::FromStr;

/// Display mode for environments.
#[derive(Debug, Default, Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Display graphical output to the user.
    ///
    /// This mode is for demonstrations and so the environment should run in as
    /// close to real time as possible and with full user interactivity enabled.
    ///
    /// The environment may also print diagnostic information to stderr.
    #[default]
    Graphical,

    /// Disable graphical output.
    ///
    /// The environment should run as quickly and quietly as possible.
    Headless,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Graphical => write!(f, "graphical"),
            Mode::Headless => write!(f, "headless"),
        }
    }
}

impl FromStr for Mode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_ascii_lowercase();
        match s.as_str() {
            "graphical" => Ok(Mode::Graphical),
            "headless" => Ok(Mode::Headless),
            _ => Err(()),
        }
    }
}

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
        let mode = mode.trim().to_ascii_lowercase();
        if mode == "graphical" {
            Mode::Graphical
        } else if mode == "headless" {
            Mode::Headless
        } else {
            panic!("Argument Error: expected either \"graphical\" or \"headless\", got \"{mode}\"");
        }
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
    return (env_spec, mode, defaults);
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
pub fn poll() -> Result<Option<Request>, JsonIoError> {
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

fn write_msg(message: &Response) -> Result<(), JsonIoError> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, message)?;
    write!(stdout, "\n")?;
    Ok(())
}

/// Acknowledge that the given message has been received and successfully acted upon.
/// The message should have originated from the `poll()` function.
pub fn ack(message: &Request) -> Result<(), JsonIoError> {
    // Birth messages don't need to be acknowledged.
    if let Request::Birth { .. } = message {
        return Ok(());
    }
    write_msg(&Response::Ack { ack: message.clone() })
}

/// Request a new individual from the evolutionary algorithm.
///
/// Argument population is optional if the environment contains exactly one population.
pub fn new(population: Option<&str>) -> Result<(), JsonIoError> {
    write_msg(&Response::New {
        population: population.map(|pop| pop.to_string()).unwrap_or(""),
    })
}

/// Request to mate two specific individuals together to produce a child individual.
///
/// Argument population is optional if the environment contains exactly one population.
pub fn mate(population: Option<&str>, parent1: u64, parent2: u64) -> Result<(), JsonIoError> {
    write_msg(&Response::Mate { parent1, parent2 })
}

/// Report an individual's score or reproductive fitness to the evolutionary algorithm.
///
/// This should be called *before* calling "death" on the individual.
///
/// Argument population is optional if the environment contains exactly one population.
/// Argument individual is optional if the environment contains exactly one individual.
pub fn score(population: Option<&str>, individual: Option<u64>, score: f64) -> Result<(), JsonIoError> {
    write_msg(&Response::Score {
        population: population.map(|pop| pop.to_string()),
        individual,
        score,
    })
}

/// Report arbitrary extraneous information about an individual to the NPC Maker program.
///
/// Argument info is a mapping of string key-value pairs.
///
/// Argument population is optional if the environment contains exactly one population.
/// Argument individual is optional if the environment contains exactly one individual.
pub fn info(
    population: Option<&str>,
    individual: Option<u64>,
    info: HashMap<String, String>,
) -> Result<(), JsonIoError> {
    write_msg(&Response::Info {
        population: population.map(|pop| pop.to_string()),
        individual,
        info,
    })
}

// Notify the evolutionary algorithm that the given individual has died.
//
// If the individual had a score or reproductive fitness then it should be
// reported using the "score()" function *before* calling this method.
///
/// Argument population is optional if the environment contains exactly one population.
/// Argument individual is optional if the environment contains exactly one individual.
pub fn death(population: Option<&str>, individual: Option<u64>) -> Result<(), JsonIoError> {
    write_msg(&Response::Death {
        population: population.map(|pop| pop.to_string()),
        individual,
    })
}

*/
