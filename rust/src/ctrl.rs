//! Controller Interface, for making and using control systems.
//!
//! Each controller runs in its own computer process and uses its standard I/O
//! channels to communicate with the environment. The interface reserves the
//! standard input and output channels its normal operations.
//! Controllers should use stderr to report any unformatted or diagnostic
//! messages (see [eprintln!()]).
//! By default, controllers inherit stderr from the environment.

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

fn _clean_path(path: impl AsRef<Path>) -> Result<PathBuf, io::Error> {
    let path = path.as_ref();
    // Expand home directory.
    let mut path_iter = path.components();
    if let Some(root) = path_iter.next() {
        if root == std::path::Component::Normal(std::ffi::OsStr::new("~")) {
            let mut path = std::env::home_dir().expect("File Error: failed to access paths relative to home directory");
            for component in path_iter {
                path.push(component);
            }
            let path = path.canonicalize()?;
            Ok(path)
        }
    } else {
        let path = path.canonicalize()?;
        Ok(path)
    }
}

/// An instance of a control system.
///
/// This structure provides methods for using controllers.
#[derive(Debug)]
pub struct Controller {
    env: PathBuf,
    pop: String,
    cmd: Vec<String>,
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
    pub fn new(environment: impl AsRef<Path>, population: &str, command: &[String]) -> Result<Self, io::Error> {
        // Clean the arguments.
        let env = _clean_path(environment)?;
        let pop = population.to_string();
        let prog = _clean_path(&command[0])?;
        let env_str = env.to_str().unwrap();
        debug_assert!(!env_str.contains("\n"));
        debug_assert!(!pop.contains("\n"));

        // Setup and run the controller command in a subprocess.
        let mut cmd = Command::new(&prog);
        cmd.args(&command[1..]);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());
        let mut ctrl = cmd.spawn()?;
        let mut stdin = BufWriter::new(ctrl.stdin.take().unwrap());
        let stdout = BufReader::new(ctrl.stdout.take().unwrap());

        //
        writeln!(stdin, "E{}", env_str)?;
        writeln!(stdin, "P{pop}")?;

        Ok(Self {
            env,
            pop,
            cmd: command.to_vec(),
            ctrl,
            stdin,
            stdout,
        })
    }

    pub fn get_environment(&self) -> &Path {
        return &self.env;
    }

    pub fn get_population(&self) -> &str {
        return &self.pop;
    }

    pub fn get_command(&self) -> &[String] {
        return &self.cmd;
    }

    /// Initialize the control system with a new genotype.  
    /// This discards the currently loaded model.  
    pub fn new_genotype(&mut self, genotype: &str) -> Result<(), io::Error> {
        debug_assert!(!genotype.contains("\n"));
        writeln!(self.stdin, "N{genotype}")?;
        Ok(())
    }

    /// Reset the control system to its initial state.
    pub fn reset(&mut self) -> Result<(), io::Error> {
        writeln!(self.stdin, "R")?;
        Ok(())
    }

    /// Advance the control system's internal state.
    pub fn advance(&mut self, dt: f64) -> Result<(), io::Error> {
        writeln!(self.stdin, "X{dt}")?;
        Ok(())
    }

    /// Write a single value to a GIN in the controller.
    pub fn set_input(&mut self, gin: u64, value: &str) -> Result<(), io::Error> {
        debug_assert!(!value.contains("\n"));
        writeln!(self.stdin, "I{gin}:{value}")?;
        Ok(())
    }

    /// Write an array of bytes to a GIN in the controller.
    pub fn set_binary(&mut self, gin: u64, value: &[u8]) -> Result<(), io::Error> {
        writeln!(self.stdin, "B{gin}:{}", value.len())?;
        self.stdin.write_all(value)?;
        Ok(())
    }

    /// Retrieve a list of outputs, as identified by their GIN.
    ///
    /// This method blocks on IO.
    pub fn get_outputs(&mut self, gin_list: &[u64]) -> Result<HashMap<u64, String>, io::Error> {
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
            let mut parts = message.splitn(2, ":");
            let gin = parts.next().unwrap();
            let value = parts.next().unwrap();
            let gin = gin.parse().unwrap();
            outputs.insert(gin, value.to_string());
        }
        Ok(outputs)
    }

    /// Save the current state of the control system to file.
    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<(), io::Error> {
        let path = path.as_ref().to_str().unwrap();
        writeln!(self.stdin, "S{path}")?;
        self.stdin.flush()?;
        Ok(())
    }
    ///  Load the state of the control system from file.
    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<(), io::Error> {
        let path = path.as_ref().to_str().unwrap();
        writeln!(self.stdin, "L{path}")?;
        Ok(())
    }

    /// Stop running the controller process.
    pub fn quit(&mut self) -> Result<(), io::Error> {
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
    New { genotype: String },
    Reset,
    Advance { dt: f64 },
    SetInput { gin: u64, value: String },
    SetBinary { gin: u64, bytes: Vec<u8> },
    GetOutput { gin: u64 },
    Save { path: PathBuf },
    Load { path: PathBuf },
    Quit,
}

impl Message {
    /// Format this message and write it to the given stream.
    pub fn write(&self, writer: &mut impl Write) -> Result<(), io::Error> {
        match self {
            Self::Environment { environment } => write!(writer, "E{}\n", environment.to_str().unwrap())?,

            Self::Population { population } => write!(writer, "P{population}\n")?,

            Self::New { genotype } => write!(writer, "N{genotype}\n")?,

            Self::Reset => write!(writer, "R\n")?,

            Self::Advance { dt } => write!(writer, "X{dt}\n")?,

            Self::SetInput { gin, value } => write!(writer, "I{gin}:{value}\n")?,

            Self::SetBinary { gin, bytes } => write!(writer, "B{gin}:{}\n", bytes.len())?,

            Self::GetOutput { gin } => write!(writer, "O{gin}\n")?,

            Self::Save { path } => write!(writer, "S{}\n", path.to_str().unwrap())?,

            Self::Load { path } => write!(writer, "L{}\n", path.to_str().unwrap())?,

            Self::Quit => write!(writer, "Q\n")?,
        };
        if let Self::SetBinary { bytes, .. } = self {
            writer.write_all(bytes.as_slice())?;
        }
        Ok(())
    }

    /// Parse the next message from the given input stream. Blocking.
    pub fn read(reader: &mut impl BufRead) -> Result<Message, io::Error> {
        let mut line = String::new();
        while line.is_empty() {
            reader.read_line(&mut line)?;
            line.pop(); // Remove the trailing newline.
        }
        let Some((msg_type, msg_body)) = line.split_at_checked(1) else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "error message"));
        };
        let msg_data = match msg_type {
            "E" => Self::Environment {
                environment: msg_body.into(),
            },
            "P" => Self::Population {
                population: msg_body.to_string(),
            },
            "N" => Self::New {
                genotype: msg_body.to_string(),
            },
            "R" => Self::Reset,
            "I" => {
                let Some((gin, value)) = msg_body.split_once(":") else {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "error message"));
                };
                Self::SetInput {
                    gin: gin.trim().parse::<u64>().unwrap(),
                    value: value.to_string(),
                }
            }
            "B" => {
                let Some((gin, num_bytes)) = msg_body.split_once(":") else {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "error message"));
                };
                let num_bytes = num_bytes.trim().parse::<usize>().unwrap();
                let mut bytes = vec![0; num_bytes];
                reader.read_exact(&mut bytes).unwrap();
                Self::SetBinary {
                    gin: gin.trim().parse::<u64>().unwrap(),
                    bytes,
                }
            }
            "X" => Self::Advance {
                dt: msg_body.parse::<f64>().unwrap(),
            },
            "O" => Self::GetOutput {
                gin: msg_body.parse::<u64>().unwrap(),
            },
            "S" => Self::Save { path: msg_body.into() },
            "L" => Self::Load { path: msg_body.into() },
            "Q" => Self::Quit,
            _ => {
                return Err(io::Error::new(io::ErrorKind::Unsupported, "error message"));
            }
        };
        Ok(msg_data)
    }
}

/// Interface for implementing controllers.
///
/// Controllers should implement this trait. Call "npc_maker::ctrl::main_loop()"
/// with an instance of the implementation to run it as a controller program.
pub trait Controller {
    fn new(&mut self, genotype: String);

    fn reset(&mut self);

    fn advance(&mut self, dt: f64);

    fn set_input(&mut self, gin: u64, value: String);

    fn set_binary(&mut self, gin: u64, bytes: Vec<u8>) {
        panic!("unsupported operation: set_binary")
    }

    fn get_output(&mut self, gin: u64) -> String;

    fn save(&mut self, path: PathBuf) {
        panic!("unsupported operation: save")
    }

    fn load(&mut self, path: PathBuf) {
        panic!("unsupported operation: load")
    }

    fn quit(&mut self) {}
}

/// Wait for the next message from the environment, for implementing controllers.
pub fn poll() -> Result<Message, io::Error> {
    Message::read(&mut io::stdin().lock())
}

/// Send an output value to the environment, for implementing controllers.
pub fn send_output(gin: u64, value: String) -> Result<(), io::Error> {
    debug_assert!(!value.contains("\n"));
    println!("{gin}:{value}");
    io::stdout().flush()?;
    Ok(())
}

/// Start the main program loop.
///
/// This method handles communications between the controller (this program) and
/// the environment. It reads and parses messages from stdin, interfaces with
/// your implementation of the Controller trait, and writes messages to stdout.
///
/// This method never returns!
pub fn main_loop(mut controller: impl Controller) -> Result<(), io::Error> {
    loop {
        let message = poll()?;
        eprintln!("CTRL-STDIN: {message:?}");
        match message {
            Message::Environment { .. } => {
                todo!()
            }
            Message::Population { .. } => {
                todo!()
            }
            Message::New { genotype } => {
                controller.new(genotype);
            }
            Message::Reset => {
                controller.reset();
            }
            Message::Advance { dt } => {
                controller.advance(dt);
            }
            Message::SetInput { gin, value } => {
                controller.set_input(gin, value);
            }
            Message::SetBinary { gin, bytes } => {
                controller.set_binary(gin, bytes);
            }
            Message::GetOutput { gin } => {
                let output = controller.get_output(gin);
                send_output(gin, output)?;
            }
            Message::Save { path } => {
                controller.save(path);
            }
            Message::Load { path } => {
                controller.load(path);
            }
            Message::Quit => {
                controller.quit();
                break;
            }
        }
    }
    Ok(())
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
            Message::New {
                genotype: "test123".to_string(),
            },
            Message::New {
                genotype: "".to_string(),
            },
            Message::New {
                genotype: "] } ){([\\n\" ".to_string(),
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
