//! Message structures for communicating between the environments and the NPC Maker.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Structure of all messages sent from the NPC Maker to the environment instances.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum Request {
    /// Request for the environment to start running.
    Start,

    /// Request for the environment to finish all work in progress.
    /// The environment may continue sending messages to the NPC Maker,
    /// but it will not be given any new individuals to evaluate.
    Stop,

    /// Request that the environment temporarily pause, with the expectation
    /// that it will later be resumed. The environment should immediately cease
    /// any computationally expensive activities, though it should retain all
    /// allocated memory.
    Pause,

    /// Request for the environment to resume after a temporary pause.
    Resume,

    /// The NPC Maker uses a watchdog timer system to manage unreliable
    /// environments. The NPC Maker periodically sends a heartbeat message to
    /// every instance of the environment and the environment must acknowledge
    /// the heartbeat in a timely manner or else the NPC Maker will assume that
    /// the environment has failed and will forcibly restart the environment.
    Heartbeat,

    /// Save the current state of the environment to the given filesystem path,
    /// including the internal state of the control systems. Note that when the
    /// environment is reloaded any in-flight messages will not be replayed.
    Save(String),

    /// Discard the current state of the environment and load a previously saved
    /// state from the given filesystem path.
    Load(String),

    /// Demand the environment shuts down and exits as fast as possible. Do not
    /// finish any work in progress and do not save any data. This instance of
    /// the environment will not be resumed. Further messages sent to the NPC
    /// Maker will be ignored.
    Quit,

    /// This message contains a new individual and its genotype. The environment
    /// should begin evaluating it immediately. This message is usually sent in
    /// response to a request for either a new individual or a mating of two
    /// individuals. This message does not need to be acknowledged.
    Birth {
        population: String,
        individual: u64,
        controller: Vec<String>,
        genotype: serde_json::Value,
    },
}

/// Structure of all messages sent from the environment instances to the NPC Maker.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged, deny_unknown_fields)]
pub enum Response {
    /// Signal that the environment is now in the given state,
    /// or acknowledge that the given request has been completed.
    Ack {
        #[serde(rename = "Ack")]
        ack: Request,
    },

    /// Request a new individual from the evolutionary algorithm.
    New {
        #[serde(rename = "New", default)]
        population: String,
    },

    /// Request to mate two individuals.
    /// Both individuals must still be alive and in the environment.
    Mate {
        #[serde(rename = "Mate")]
        parents: Vec<u64>,
    },

    /// Report the score or reproductive fitness of an individual.
    Score {
        #[serde(rename = "Score")]
        score: f64,
        individual: u64,
    },

    /// Associate some extra information with an individual. The data is kept
    /// alongside the individual in perpetuity and is displayed to the user.
    Info {
        #[serde(rename = "Info")]
        info: HashMap<String, String>,
        individual: u64,
    },

    /// Report the death of an individual.
    Death {
        #[serde(rename = "Death")]
        individual: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_roundtrip() {
        let all_requests = [
            Request::Start,
            Request::Stop,
            Request::Pause,
            Request::Resume,
            Request::Heartbeat,
            Request::Save("/foo/ b a r /my_save.json".to_string()),
            Request::Load("./my_save.json".to_string()),
            Request::Quit,
            Request::Birth {
                population: "pop1".to_string(),
                individual: 42,
                controller: vec![
                    "~/mycode/ashdji;f.exe".to_string(),
                    "".to_string(),
                    " ".to_string(),
                    ",.,<>.,.,.,><>,".to_string(),
                ],
                genotype: serde_json::json!([]),
            },
            Request::Birth {
                population: "pop1".to_string(),
                individual: 43,
                controller: vec![],
                genotype: serde_json::json!([{}, {}, {}]),
            },
        ];
        let mut info = HashMap::new();
        info.insert("my_key".to_string(), "my_value".to_string());
        let mut all_responses = vec![
            Response::New { population: None },
            Response::New {
                population: Some("my pop1".to_string()),
            },
            Response::New {
                population: Some(" ".to_string()),
            },
            Response::Mate {
                population: None,
                parent1: 5,
                parent2: 7,
            },
            Response::Mate {
                population: Some("pop 3".to_string()),
                parent1: 5,
                parent2: 8,
            },
            Response::Score {
                population: None,
                individual: Some(42),
                score: 42.2,
            },
            Response::Score {
                population: Some("ends with a number 3".to_string()),
                individual: Some(21),
                score: 7.7,
            },
            Response::Info {
                population: None,
                individual: Some(101),
                info: HashMap::new(),
            },
            Response::Info {
                population: None,
                individual: None,
                info: HashMap::new(),
            },
            Response::Info {
                population: None,
                individual: None,
                info: info.clone(),
            },
            Response::Info {
                population: Some("pop10".to_string()),
                individual: Some(85),
                info: info.clone(),
            },
            Response::Death {
                population: None,
                individual: Some(99),
            },
            Response::Death {
                population: Some("2".to_string()),
                individual: Some(99),
            },
        ];
        for msg in &all_requests {
            all_responses.push(Response::Ack(msg.clone()));
        }

        println!("REQUESTS:");
        for msg in all_requests {
            let json = serde_json::to_string(&msg).unwrap();
            dbg!(&json);
            let recv = serde_json::from_str(&json).unwrap();
            assert_eq!(msg, recv);
            assert!(!json.contains("\n"));
        }
        println!("RESPONSES:");
        for msg in all_responses {
            let json = serde_json::to_string(&msg).unwrap();
            dbg!(&json);
            let recv = serde_json::from_str(&json).unwrap();
            assert_eq!(msg, recv);
            assert!(!json.contains("\n"));
        }
    }

    /// Check that the messages being sent to the environment are exactly as expected.
    #[test]
    fn send_string() {
        assert_eq!(serde_json::to_string(&Request::Start).unwrap(), "\"Start\"");
        assert_eq!(serde_json::to_string(&Request::Stop).unwrap(), "\"Stop\"");
        assert_eq!(serde_json::to_string(&Request::Pause).unwrap(), "\"Pause\"");
        assert_eq!(serde_json::to_string(&Request::Resume).unwrap(), "\"Resume\"");
        assert_eq!(serde_json::to_string(&Request::Heartbeat).unwrap(), "\"Heartbeat\"");
        assert_eq!(serde_json::to_string(&Request::Quit).unwrap(), "\"Quit\"");

        assert_eq!(
            serde_json::to_string(&Request::Save("foobar".to_string())).unwrap(),
            r#"{"Save":"foobar"}"#
        );
        assert_eq!(
            serde_json::to_string(&Request::Load("foobar".to_string())).unwrap(),
            r#"{"Load":"foobar"}"#
        );
        assert_eq!(
            serde_json::to_string(&Request::Birth {
                population: "pop1".to_string(),
                individual: 1234,
                controller: vec!["/usr/bin/q".to_string()],
                genotype: serde_json::json! {
                    [
                        {"name": 6, "type": "foo"},
                        {"name": 7, "type": "bar"},
                    ]
                },
            })
            .unwrap(),
            r#"{"Birth":{"population":"pop1","individual":1234,"controller":["/usr/bin/q"],"genotype":[{"name":6,"type":"foo"},{"name":7,"type":"bar"}]}}"#
        );
    }
}
