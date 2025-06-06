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

    /// This message contains a new individual and its genome. The environment
    /// should begin evaluating it immediately. This message is usually sent in
    /// response to a request for either a new individual or a mating of two
    /// individuals. This message does not need to be acknowledged.
    Birth {
        name: String,

        #[serde(default)]
        population: String,

        #[serde(default)]
        parents: Vec<String>,

        controller: Vec<String>,

        genome: serde_json::Value,
    },

    /// Send a user defined message to the environment.
    Custom(serde_json::Value),
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
        parents: [String; 2],
    },

    /// Report the score or reproductive fitness of an individual.
    Score {
        #[serde(rename = "Score")]
        score: String,
        name: String,
    },

    /// Associate some extra information with an individual.
    Info {
        #[serde(rename = "Info")]
        info: HashMap<String, String>,
        name: String,
    },

    /// Report the death of an individual.
    Death {
        #[serde(rename = "Death")]
        name: String,
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
                name: "42".to_string(),
                controller: vec![
                    "~/mycode/ashdji;f.exe".to_string(),
                    "".to_string(),
                    " ".to_string(),
                    ",.,<>.,.,.,><>,".to_string(),
                ],
                genome: serde_json::json!([]),
                parents: vec!["qewrty".to_string(), "".to_string()],
            },
            Request::Birth {
                population: "pop1".to_string(),
                name: "43".to_string(),
                controller: vec![],
                genome: serde_json::json!([{}, {}, {}]),
                parents: vec![
                    "5883654456843513551325647448544554151".to_string(),
                    "43848588658686835437723784328734587934598859348954".to_string(),
                ],
            },
        ];
        let mut info = HashMap::new();
        info.insert("my_key".to_string(), "my_value".to_string());
        let mut all_responses = vec![
            Response::New {
                population: "".to_string(),
            },
            Response::New {
                population: "my pop1".to_string(),
            },
            Response::New {
                population: " ".to_string(),
            },
            Response::Mate {
                parents: ["5".to_string(), "7".to_string()],
            },
            Response::Mate {
                parents: ["5".to_string(), "8".to_string()],
            },
            Response::Score {
                name: "42".to_string(),
                score: "42.2".to_string(),
            },
            Response::Score {
                name: "21".to_string(),
                score: "7.7".to_string(),
            },
            Response::Info {
                name: "101".to_string(),
                info: HashMap::new(),
            },
            Response::Info {
                name: "".to_string(),
                info: HashMap::new(),
            },
            Response::Info {
                name: "".to_string(),
                info: info.clone(),
            },
            Response::Info {
                name: "85".to_string(),
                info: info.clone(),
            },
            Response::Death { name: "".to_string() },
            Response::Death { name: "99".to_string() },
        ];
        for msg in &all_requests {
            all_responses.push(Response::Ack { ack: msg.clone() });
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
                name: "1234".to_string(),
                population: "pop1".to_string(),
                parents: vec!["1020".to_string(), "1077".to_string()],
                controller: vec!["/usr/bin/q".to_string()],
                genome: serde_json::json! {
                    [
                        {"name": 6, "type": "foo"},
                        {"name": 7, "type": "bar"},
                    ]
                },
            })
            .unwrap(),
            r#"{"Birth":{"name":"1234","population":"pop1","parents":["1020","1077"],"controller":["/usr/bin/q"],"genome":[{"name":6,"type":"foo"},{"name":7,"type":"bar"}]}}"#
        );
        assert_eq!(
            serde_json::to_string(&Request::Custom(serde_json::json!({"foo":"bar"}))).unwrap(),
            r#"{"Custom":{"foo":"bar"}}"#
        );
    }

    /// Check that the messages received from the environment are exactly as expected.
    #[test]
    fn recv_string() {
        assert_eq!(
            serde_json::to_string(&Response::Ack { ack: Request::Start }).unwrap(),
            "{\"Ack\":\"Start\"}"
        );
        assert_eq!(
            serde_json::to_string(&Response::New {
                population: String::new()
            })
            .unwrap(),
            "{\"New\":\"\"}"
        );
        assert_eq!(
            serde_json::to_string(&Response::New {
                population: "pop1".to_string()
            })
            .unwrap(),
            "{\"New\":\"pop1\"}"
        );
        assert_eq!(
            serde_json::to_string(&Response::Mate {
                parents: ["parent1".to_string(), "parent2".to_string()]
            })
            .unwrap(),
            "{\"Mate\":[\"parent1\",\"parent2\"]}"
        );
        assert_eq!(
            serde_json::to_string(&Response::Score {
                name: "xyz".to_string(),
                score: "-3.7".to_string(),
            })
            .unwrap(),
            "{\"Score\":\"-3.7\",\"name\":\"xyz\"}"
        );
        assert_eq!(
            serde_json::to_string(&Response::Info {
                name: "abcd".to_string(),
                info: HashMap::new()
            })
            .unwrap(),
            "{\"Info\":{},\"name\":\"abcd\"}"
        );
        assert_eq!(
            serde_json::to_string(&Response::Death { name: String::new() }).unwrap(),
            "{\"Death\":\"\"}"
        );
        assert_eq!(
            serde_json::to_string(&Response::Death {
                name: "abc".to_string()
            })
            .unwrap(),
            "{\"Death\":\"abc\"}"
        );
    }
}
