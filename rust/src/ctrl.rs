//! Controller Interface, for making and using control systems.
//!
//! Each controller runs in its own computer process and uses its standard I/O
//! channels to communicate with the environment. The interface reserves the
//! standard input and output channels for its normal operations.
//! Controllers should use stderr to report any unformatted or diagnostic
//! messages (see [eprintln!()]).
//! By default, controllers inherit stderr from the environment.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, BufWriter, Error, ErrorKind, Result, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

fn _clean_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();
    // Expand home directory.
    let mut path_iter = path.components();
    if let Some(root) = path_iter.next() {
        if root == std::path::Component::Normal(std::ffi::OsStr::new("~")) {
            let mut path = std::env::home_dir().expect("File Error: failed to find the user's home directory");
            path.extend(path_iter);
            let path = path.canonicalize()?;
            return Ok(path);
        }
    }
    let path = path.canonicalize()?;
    Ok(path)
}

/// An instance of a control system.
///
/// This object provides methods for using a controller.
#[derive(Debug)]
pub struct Controller {
    environment: PathBuf,
    population: String,
    command: Vec<String>,
    ctrl: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl Controller {
    /// Argument environment is the file path of the current environment specification file.
    ///
    /// Argument population is a name and a key into the environment spec's "populations" table.
    ///
    /// Argument command is the command line invocation for the controller program.  
    /// The first string in the list is the program, the remaining strings are its command line arguments.  
    pub fn new(environment: impl AsRef<Path>, population: &str, command: Vec<String>) -> Result<Self> {
        // Clean the arguments.
        let environment = _clean_path(environment)?;
        let population = population.to_string();
        let program = _clean_path(&command[0])?;
        let environment_path = environment.to_str().unwrap();
        debug_assert!(!environment_path.contains('\n'));
        debug_assert!(!population.contains('\n'));

        // Setup and run the controller command in a subprocess.
        let mut cmd = Command::new(&program);
        cmd.args(&command[1..]);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());
        let mut ctrl = cmd.spawn()?;
        let mut stdin = BufWriter::new(ctrl.stdin.take().unwrap());
        let stdout = BufReader::new(ctrl.stdout.take().unwrap());

        //
        writeln!(stdin, "E{}", environment_path)?;
        writeln!(stdin, "P{population}")?;

        Ok(Self {
            environment,
            population,
            command,
            ctrl,
            stdin,
            stdout,
        })
    }

    pub fn get_environment(&self) -> &Path {
        &self.environment
    }

    pub fn get_population(&self) -> &str {
        &self.population
    }

    pub fn get_command(&self) -> &[String] {
        &self.command
    }

    pub fn is_alive(&mut self) -> Result<bool> {
        let exit_status_code = self.ctrl.try_wait()?;
        Ok(exit_status_code.is_none())
    }

    /// Initialize the control system with a new genome.  
    /// This discards the currently loaded model.  
    ///
    /// Argument value must be a single line.  
    pub fn genome(&mut self, value: &str) -> Result<()> {
        debug_assert!(!value.contains('\n'));
        writeln!(self.stdin, "G{value}")?;
        Ok(())
    }

    /// Reset the control system to its initial state.
    pub fn reset(&mut self) -> Result<()> {
        writeln!(self.stdin, "R")?;
        Ok(())
    }

    /// Advance the control system's internal state.
    pub fn advance(&mut self, dt: f64) -> Result<()> {
        writeln!(self.stdin, "X{dt}")?;
        Ok(())
    }

    /// Write a single value to a GIN in the controller.
    pub fn set_input(&mut self, gin: u64, value: &str) -> Result<()> {
        debug_assert!(!value.contains('\n'));
        writeln!(self.stdin, "I{gin}:{value}")?;
        Ok(())
    }

    /// Write an array of bytes to a GIN in the controller.
    pub fn set_binary(&mut self, gin: u64, value: &[u8]) -> Result<()> {
        writeln!(self.stdin, "B{gin}:{}", value.len())?;
        self.stdin.write_all(value)?;
        Ok(())
    }

    /// Retrieve a list of outputs, as identified by their GIN.
    ///
    /// This method blocks on IO.
    pub fn get_outputs(&mut self, gin_list: &[u64]) -> Result<HashMap<u64, String>> {
        // Request the outputs.
        for gin in gin_list {
            writeln!(self.stdin, "O{gin}")?;
        }
        self.stdin.flush()?;
        // Wait for the controller to respond.
        let mut outputs = HashMap::<u64, String>::new();
        let mut message = String::new();
        while outputs.len() < gin_list.len() {
            message.clear();
            self.stdout.read_line(&mut message)?;
            message.pop(); // Discard the trailing newline.
            let (gin, value) = message.split_once(":").unwrap();
            let gin = gin.parse().unwrap();
            outputs.insert(gin, value.to_string());
        }
        Ok(outputs)
    }

    /// Save the current state of the control system to file.
    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_str().unwrap();
        debug_assert!(!path.contains('\n'));
        writeln!(self.stdin, "S{path}")?;
        self.stdin.flush()?;
        Ok(())
    }

    ///  Load the state of the control system from file.
    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_str().unwrap();
        debug_assert!(!path.contains('\n'));
        writeln!(self.stdin, "L{path}")?;
        Ok(())
    }

    /// Send a custom message to the controller using a new message type.
    pub fn custom(&mut self, message_type: char, message_body: &str) -> Result<()> {
        debug_assert!(message_type == message_type.to_ascii_uppercase());
        debug_assert!(!"EPGRXIBOSLQ".contains(message_type));
        debug_assert!(!message_body.contains('\n'));
        writeln!(self.stdin, "{message_type}{message_body}")?;
        Ok(())
    }

    /// Stop running the controller process.
    pub fn quit(&mut self) -> Result<()> {
        writeln!(self.stdin, "Q")?;
        self.stdin.flush()?;
        Ok(())
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        let _ = self.quit();
    }
}

/// Structure of all messages sent from environments to controllers.
///
/// These messages are transmitted over the controller stdin channel.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Environment { environment: PathBuf },
    Population { population: String },
    Genome { value: String },
    Reset,
    Advance { dt: f64 },
    SetInput { gin: u64, value: String },
    SetBinary { gin: u64, bytes: Vec<u8> },
    GetOutput { gin: u64 },
    Save { path: PathBuf },
    Load { path: PathBuf },
    Custom { message_type: char, body: String },
    Quit,
}

impl Message {
    /// Format this message and write it to the given stream.
    pub fn write(&self, writer: &mut impl Write) -> Result<()> {
        match self {
            Self::Environment { environment } => writeln!(writer, "E{}", environment.to_str().unwrap())?,

            Self::Population { population } => writeln!(writer, "P{population}")?,

            Self::Genome { value } => writeln!(writer, "G{value}")?,

            Self::Reset => writeln!(writer, "R")?,

            Self::Advance { dt } => writeln!(writer, "X{dt}")?,

            Self::SetInput { gin, value } => writeln!(writer, "I{gin}:{value}")?,

            Self::SetBinary { gin, bytes } => writeln!(writer, "B{gin}:{}", bytes.len())?,

            Self::GetOutput { gin } => writeln!(writer, "O{gin}")?,

            Self::Save { path } => writeln!(writer, "S{}", path.to_str().unwrap())?,

            Self::Load { path } => writeln!(writer, "L{}", path.to_str().unwrap())?,

            Self::Custom { message_type, body } => writeln!(writer, "{}{}", message_type, body)?,

            Self::Quit => writeln!(writer, "Q")?,
        };
        if let Self::SetBinary { bytes, .. } = self {
            writer.write_all(bytes.as_slice())?;
        }
        Ok(())
    }

    /// Parse the next message from the given input stream. Blocking.
    pub fn read(reader: &mut impl BufRead) -> Result<Message> {
        let mut line = String::new();
        while line.is_empty() {
            let bytes_read = reader.read_line(&mut line)?;
            if bytes_read == 0 {
                return Err(Error::new(ErrorKind::UnexpectedEof, "stdin closed"));
            }
            line.pop(); // Remove the trailing newline.
        }
        let msg_type = line.chars().next().unwrap();
        let msg_body = &line[msg_type.len_utf8()..];
        let msg_data = match msg_type.to_ascii_uppercase() {
            'E' => Self::Environment {
                environment: msg_body.into(),
            },
            'P' => Self::Population {
                population: msg_body.to_string(),
            },
            'G' => Self::Genome {
                value: msg_body.to_string(),
            },
            'R' => Self::Reset,
            'I' => {
                let Some((gin, value)) = msg_body.split_once(":") else {
                    return Err(Error::new(ErrorKind::InvalidData, "error message"));
                };
                Self::SetInput {
                    gin: gin.trim().parse::<u64>().unwrap(),
                    value: value.to_string(),
                }
            }
            'B' => {
                let Some((gin, num_bytes)) = msg_body.split_once(":") else {
                    return Err(Error::new(ErrorKind::InvalidData, "error message"));
                };
                let num_bytes = num_bytes.trim().parse::<usize>().unwrap();
                let mut bytes = vec![0; num_bytes];
                reader.read_exact(&mut bytes).unwrap();
                Self::SetBinary {
                    gin: gin.trim().parse::<u64>().unwrap(),
                    bytes,
                }
            }
            'X' => Self::Advance {
                dt: msg_body.parse::<f64>().unwrap(),
            },
            'O' => Self::GetOutput {
                gin: msg_body.parse::<u64>().unwrap(),
            },
            'S' => Self::Save { path: msg_body.into() },
            'L' => Self::Load { path: msg_body.into() },
            'Q' => Self::Quit,
            _ => Self::Custom {
                message_type: msg_type,
                body: msg_body.to_string(),
            },
        };
        Ok(msg_data)
    }
}

/// Wait for the next message from the environment, for implementing controllers.
fn poll() -> Result<Message> {
    Message::read(&mut std::io::stdin().lock())
}

/// Send an output value to the environment, for implementing controllers.
fn output(gin: u64, value: String) -> Result<()> {
    debug_assert!(!value.contains('\n'));
    println!("{gin}:{value}");
    std::io::stdout().flush()?;
    Ok(())
}

// Store these in global variables so that the main function is can be re-entered in case of error.
static ENVIRONMENT: Mutex<Option<PathBuf>> = Mutex::new(None);
static POPULATION: Mutex<Option<String>> = Mutex::new(None);

/// Interface for implementing controllers.
#[allow(unused_variables)]
pub trait API {
    /// Run a controller program.
    ///
    /// This method handles communications between the controller (this program) and
    /// the environment. It reads and parses messages from stdin, interfaces with
    /// your implementation of the API trait, and writes messages to stdout.
    ///
    /// This method never returns!
    fn main(&mut self) -> Result<()> {
        loop {
            let message = poll()?;
            match message {
                Message::Environment { environment } => {
                    ENVIRONMENT.lock().unwrap().replace(environment);
                }
                Message::Population { population } => {
                    POPULATION.lock().unwrap().replace(population);
                }
                Message::Genome { value } => {
                    // Wait for the locks.
                    let environment_lock = ENVIRONMENT.lock().unwrap();
                    let population_lock = POPULATION.lock().unwrap();
                    // Borrow the data.
                    let environment = environment_lock.as_ref();
                    let population = population_lock.as_ref();
                    // Fill in missing values.
                    let environment = match environment {
                        Some(ref_path_buf) => ref_path_buf.as_path(),
                        None => Path::new(""),
                    };
                    let population = match population {
                        Some(ref_string) => ref_string.as_str(),
                        None => "",
                    };
                    self.genome(environment, population, value);
                }
                Message::Reset => {
                    self.reset();
                }
                Message::Advance { dt } => {
                    self.advance(dt);
                }
                Message::SetInput { gin, value } => {
                    self.set_input(gin, value);
                }
                Message::SetBinary { gin, bytes } => {
                    self.set_binary(gin, bytes);
                }
                Message::GetOutput { gin } => {
                    let value = self.get_output(gin);
                    output(gin, value)?;
                }
                Message::Save { path } => {
                    self.save(path);
                }
                Message::Load { path } => {
                    self.load(path);
                }
                Message::Custom { message_type, body } => {
                    self.custom(message_type, &body);
                }
                Message::Quit => {
                    self.quit();
                    break;
                }
            }
        }
        Ok(())
    }

    /// Discard the current model and load a new one.
    ///
    /// The environment and population will not change during the lifetime of
    /// the controller's computer process.
    ///
    /// Argument value is the parameters for the new control system.
    fn genome(&mut self, environment: &Path, population: &str, value: String);

    /// Reset the currently loaded model to it's initial state.
    fn reset(&mut self);

    /// Argument dt is the time period to advance over, measured in seconds.
    fn advance(&mut self, dt: f64);

    /// Receive data from the environment into the controller.
    fn set_input(&mut self, gin: u64, value: String);

    /// Receive an array of bytes from the environment into the controller.
    ///
    /// Optional, panics by default.
    fn set_binary(&mut self, gin: u64, bytes: Vec<u8>) {
        panic!("unsupported operation: set_binary")
    }

    /// The environment has requested that the controller send it an output.
    fn get_output(&mut self, gin: u64) -> String;

    /// Save the current state of the controller to file.
    ///
    /// Optional, panics by default.
    fn save(&mut self, path: PathBuf) {
        panic!("unsupported operation: save")
    }

    /// Load the state of a controller from file.
    ///
    /// Optional, panics by default.
    fn load(&mut self, path: PathBuf) {
        panic!("unsupported operation: load")
    }

    /// Receive a custom message from the controller using a new message type.
    ///
    /// Optional, panics by default.
    fn custom(&mut self, message_type: char, message_body: &str) {
        panic!("unsupported operation: custom")
    }

    /// Optional.
    ///
    /// This method is called just before the controller process exits.
    fn quit(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_roundtrip() {
        let test_messages = [
            Message::Environment {
                environment: PathBuf::from("test/jungle123"),
            },
            Message::Environment {
                environment: PathBuf::from(""),
            },
            Message::Environment {
                environment: PathBuf::from(" / \" _^ .?`~@!#$%^&*()_+-=[{]};:',<.>/? "),
            },
            Message::Population {
                population: "zebra".to_string(),
            },
            Message::Population {
                population: "".to_string(),
            },
            Message::Population {
                population: " ".to_string(),
            },
            //
            Message::Genome {
                value: "test123".to_string(),
            },
            Message::Genome { value: "".to_string() },
            Message::Genome {
                value: "] } ){([\\n\" ".to_string(),
            },
            //
            Message::Reset,
            //
            Message::Advance { dt: 0.123 },
            Message::Advance { dt: -0.123 },
            Message::Advance { dt: 0.0 },
            Message::Advance { dt: 123456789e12 },
            //
            Message::SetInput {
                gin: 42,
                value: "42".to_string(),
            },
            Message::SetInput {
                gin: 43,
                value: "-1234.56e-4".to_string(),
            },
            // Test that single strings are processed exactly as they are.
            Message::SetInput {
                gin: 44,
                value: "".to_string(),
            },
            Message::SetInput {
                gin: 45,
                value: " ".to_string(),
            },
            Message::SetInput {
                gin: 46,
                value: ": ".to_string(),
            },
            Message::SetInput {
                gin: 47,
                value: "\t".to_string(),
            },
            Message::SetInput {
                gin: 48,
                value: "~!@#$%^&*()_+-={}[]:'<>,./?|".to_string(),
            },
            // Test that it does NOT parse quotes.
            Message::SetInput {
                gin: 50,
                value: r#"""#.to_string(),
            },
            // Test that it does NOT interpret backslashes as escapes.
            Message::SetInput {
                gin: 49,
                value: r#"\"#.to_string(),
            },
            Message::SetInput {
                gin: 50,
                value: r#"\n"#.to_string(),
            },
            //
            Message::SetBinary {
                gin: 100,
                bytes: b"123456789".to_vec(),
            },
            Message::SetBinary {
                gin: 100,
                bytes: b"".to_vec(),
            },
            Message::SetBinary {
                gin: 100,
                bytes: b" ".to_vec(),
            },
            Message::SetBinary {
                gin: 100,
                bytes: b":".to_vec(),
            },
            Message::SetBinary {
                gin: 100,
                bytes: b"\"".to_vec(),
            },
            Message::SetBinary {
                gin: 100,
                bytes: b"\\".to_vec(),
            },
            Message::SetBinary {
                gin: 100,
                bytes: b"\\n".to_vec(),
            },
            Message::SetBinary {
                gin: 100,
                bytes: b"1234".to_vec(),
            },
            //
            Message::GetOutput { gin: 0 },
            Message::GetOutput { gin: 100 },
            Message::GetOutput { gin: u64::MAX },
            //
            Message::Save {
                path: PathBuf::from("/tmp/my_save_file,"),
            },
            //
            Message::Load {
                path: PathBuf::from("\\tmp\\my_save_file."),
            },
            //
            Message::Custom {
                message_type: '~',
                body: ":hello custom message:".to_string(),
            },
            Message::Custom {
                message_type: '$',
                body: "".to_string(),
            },
            Message::Custom {
                message_type: '!',
                body: " \t ".to_string(),
            },
            //
            Message::Quit,
        ];

        for original in test_messages {
            let mut message = vec![];
            original.write(&mut message).unwrap();
            let returned = Message::read(&mut message.as_slice()).unwrap();
            assert_eq!(original, returned);
        }
    }
}
