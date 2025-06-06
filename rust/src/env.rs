//! Environment Interface, for making and using environments.
//!
//! Each environment runs in its own computer process and uses stdin & stdout to
//! communicate with the evolutionary algorithm and the main NPC Maker program.
//! Environments should use stderr to report any unformatted or diagnostic messages
//! (see [eprintln!()]).

mod api;
mod messages;
mod specification;

pub use api::{ack, death, get_args, info, mate, new, poll, score};
pub use messages::{Request, Response};
pub use specification::{EnvironmentSpec, InterfaceSpec, PopulationSpec, SettingsSpec};

use process_anywhere::{Computer, Process};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Json(#[from] serde_json::Error),

    #[error("SSH: {0}")]
    Ssh(ssh2::Error),

    #[error("{0}")]
    Utf8(std::string::FromUtf8Error),

    #[error("Utf8 error in path: {0}")]
    Utf8Path(PathBuf),
}

impl From<process_anywhere::Error> for Error {
    fn from(error: process_anywhere::Error) -> Self {
        match error {
            process_anywhere::Error::Io(error) => Error::Io(error),
            process_anywhere::Error::Ssh(error) => Error::Ssh(error),
            process_anywhere::Error::Utf8(error) => Error::Utf8(error),
        }
    }
}

/// Display mode for environments.
#[derive(Debug, Default, Serialize, Deserialize, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Disable graphical output.
    ///
    /// The environment should run as quickly and quietly as possible.
    Headless,

    /// Display graphical output to the user.
    ///
    /// This mode is for demonstrations and so the environment should run in as
    /// close to real time as possible and with full user interactivity enabled.
    ///
    /// The environment may also print diagnostic information to stderr.
    #[default]
    Graphical,
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
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_ascii_lowercase();
        match s.as_str() {
            "graphical" => Ok(Mode::Graphical),
            "headless" => Ok(Mode::Headless),
            _ => Err(format!("expected either \"graphical\" or \"headless\", got \"{s}\"")),
        }
    }
}

impl From<bool> for Mode {
    /// Converts false to headless,  
    /// Converts true to graphical.  
    fn from(mode: bool) -> Self {
        if mode {
            Mode::Graphical
        } else {
            Mode::Headless
        }
    }
}

/// An instance of an environment.
///
/// Each environment instance execute in its own subprocess
/// and communicates with the caller over its standard I/O channels.
pub struct Environment {
    env_spec: Arc<EnvironmentSpec>,
    mode: Mode,
    settings: HashMap<String, String>,
    process: Box<Process>,
    outstanding: HashMap<String, Individual>,
}

struct Individual {
    score: Option<f64>,
    info: HashMap<String, String>,
}

impl Environment {
    /// Start running an environment program.
    ///
    /// Argument populations is a dict of evolution API instances, indexed by
    /// population name. Every population must have a corresponding instance of
    /// npc_maker.evo.API.
    ///
    /// Argument env_spec is the filesystem path of the environment specification.
    ///
    /// Argument mode is either the word "graphical" or the word "headless" to
    /// indicate whether or not the environment should show graphical output to
    /// the user.
    ///
    /// Argument settings is a dict of command line arguments for the
    /// environment process. These must match what is listed in the environment
    /// specification.
    pub fn new(
        computer: Arc<Computer>,
        env_spec: Arc<EnvironmentSpec>,
        mode: Mode,
        settings: HashMap<String, String>,
    ) -> Result<Self, Error> {
        // Assemble the program's command line invocation.
        let mut command = Vec::<&str>::with_capacity(2 + 2 * settings.len());
        //
        let Some(path) = env_spec.path.as_path().to_str() else {
            return Err(Error::Utf8Path(env_spec.path.clone()));
        };
        command.push(path);
        //
        let Some(spec) = env_spec.spec.as_path().to_str() else {
            return Err(Error::Utf8Path(env_spec.spec.clone()));
        };
        command.push(spec);
        //
        match mode {
            Mode::Graphical => command.push("graphical"),
            Mode::Headless => command.push("headless"),
        }
        //
        let arena = typed_arena::Arena::<String>::new();
        for item in env_spec.settings.iter() {
            command.push(item.name());
            if let Some(argument) = settings.get(item.name()) {
                command.push(argument.as_str());
            } else {
                let default = arena.alloc(item.default());
                command.push(default.as_str());
            }
        }
        //
        let process = computer.exec(&command)?;
        //
        Ok(Self {
            env_spec,
            mode,
            settings,
            process,
            outstanding: HashMap::new(),
        })
    }

    /// Get the environment specification.
    pub fn get_env_spec(&self) -> &Arc<EnvironmentSpec> {
        &self.env_spec
    }

    /// Get the output display mode argument.
    pub fn get_mode(&self) -> Mode {
        self.mode
    }

    /// Get the "settings" argument.
    pub fn get_settings(&self) -> &HashMap<String, String> {
        &self.settings
    }

    /// Is the environment subprocess still running?
    pub fn is_alive(&mut self) -> Result<bool, Error> {
        Ok(self.process.is_alive()?)
    }

    /// Request to start the environment.
    pub fn start(&mut self) -> Result<(), Error> {
        self.process.send_line(r#""Start""#)?;
        Ok(())
    }

    /// Request to stop the environment.
    pub fn stop(&mut self) -> Result<(), Error> {
        self.process.send_line(r#""Stop""#)?;
        Ok(())
    }

    /// Request to pause the environment.
    pub fn pause(&mut self) -> Result<(), Error> {
        self.process.send_line(r#""Pause""#)?;
        Ok(())
    }

    /// Request to resume the environment.
    pub fn resume(&mut self) -> Result<(), Error> {
        self.process.send_line(r#""Resume""#)?;
        Ok(())
    }

    /// Request to quit the environment.
    pub fn quit(&mut self) -> Result<(), Error> {
        self.process.send_line(r#""Quit""#)?;
        Ok(())
    }

    /// Request to save the environment to the given path.
    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<(), Error> {
        let Some(path) = path.as_ref().to_str() else {
            return Err(Error::Utf8Path(path.as_ref().to_path_buf()));
        };
        self.process.send_line(&format!(r#"{{"Save":"{path}"}}"#))?;
        Ok(())
    }

    /// Request to load the environment from the given path.
    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<(), Error> {
        let Some(path) = path.as_ref().to_str() else {
            return Err(Error::Utf8Path(path.as_ref().to_path_buf()));
        };
        self.process.send_line(&format!(r#"{{"Load":"{path}"}}"#))?;
        Ok(())
    }

    /// Send a user defined JSON message to the environment.
    pub fn custom(&mut self, message: &str) -> Result<(), Error> {
        debug_assert!(!message.contains('\n'));
        self.process.send_line(&format!(r#"{{"Custom":{message}}}"#))?;
        Ok(())
    }

    ///
    pub fn birth(
        &mut self,
        name: &str,
        parents: &[&str],
        population: &str,
        controller: &[&str],
        genome: &str,
    ) -> Result<(), Error> {
        debug_assert!(!name.is_empty());
        debug_assert!(!name.contains('\n'));
        debug_assert!(!controller.is_empty());
        debug_assert!(controller.iter().all(|x| !x.contains('\n')));
        debug_assert!(parents.iter().all(|x| !x.contains('\n')));
        debug_assert!(!genome.contains('\n'));
        //
        let env = &self.env_spec.name;
        // Fill in the population if there is exactly one.
        let pop = if !population.is_empty() {
            assert!(
                self.env_spec
                    .populations
                    .iter()
                    .find(|pop| pop.name == population)
                    .is_some(),
                "no such population"
            );
            population
        } else {
            assert!(self.env_spec.populations.len() == 1, "missing argument \"population\"");
            &self.env_spec.populations[0].name
        };
        let ctrl_json = serde_json::to_string(controller).unwrap();
        let parents_json = serde_json::to_string(parents).unwrap();
        //
        let name_conflict = self.outstanding.insert(
            name.to_string(),
            Individual {
                score: None,
                info: HashMap::new(),
            },
        );
        assert!(name_conflict.is_none(), "individuals with duplicate names");
        //
        let message = &format!(
            r#"{{"Birth":{{"environment":"{env}","population":"{pop}","name":"UUID","controller":{ctrl_json},"genome":{genome},"parents":{parents_json}}}}}"#
        );
        self.process.send_line(message)?;
        Ok(())
    }

    ///
    pub fn poll(&mut self) -> Result<Option<Response>, Error> {
        //
        while let Some(line) = self.process.error_line()? {
            eprintln!("{line}");
        }
        // Get the next message.
        let Some(message) = self.process.recv_line()? else {
            return Ok(None);
        };
        // Ignore empty lines.
        let message = message.trim();
        if message.is_empty() {
            return Ok(None);
        }
        // Parse the line into the message structure.
        let message: Response = serde_json::from_str(message)?;

        Ok(Some(message))

        /*
        // Process the message.
        match message {
            Response::Ack { .. } => {
                //
                Ok(Some(message))
            }
            Response::New { .. } => {
                //
                Ok(Some(message))
            }
            Response::Mate { .. } => {
                //
                Ok(Some(message))
            }
            Response::Score { .. } => {
                //
                Ok(None)
            }
            Response::Info { .. } => {
                //
                Ok(None)
            }
            Response::Death { .. } => {
                //
                Ok(Some(message))
            }
        }
        */
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        //
    }
}
