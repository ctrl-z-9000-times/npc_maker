//! Message structures for communicating between the environments and the NPC Maker.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Structure of all messages sent from the NPC Maker to the environment instances.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum Request {
    /// The environment's standard input channel is closed.
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
    }

    /// Check that the messages received from the environment are exactly as expected.
    #[test]
    fn recv_string() {
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
