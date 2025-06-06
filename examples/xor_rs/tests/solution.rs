use npc_maker::env::{Environment, EnvironmentSpec, Mode, Response};
use std::path::Path;
use std::sync::Arc;

#[test]
fn solution() {
    let solution: String = serde_json::json! {[
        {"name": 0, "type": "Node", "slope": 2.0, "midpoint":  0.5},
        {"name": 1, "type": "Node", "slope": 2.0, "midpoint":  0.5},
        {"name": 2, "type": "Node", "slope": 2.0, "midpoint":  0.5},
        {"name": 3, "type": "Node", "slope": 2.0, "midpoint":  2.0},
        {"name": 6, "type": "Edge", "presyn": 0, "postsyn": 2, "weight": 1.0},
        {"name": 7, "type": "Edge", "presyn": 1, "postsyn": 2, "weight": 1.0},
        {"name": 8, "type": "Edge", "presyn": 3, "postsyn": 2, "weight": -4.0},
        {"name": 10, "type": "Edge", "presyn": 0, "postsyn": 3, "weight": 1.0},
        {"name": 11, "type": "Edge", "presyn": 1, "postsyn": 3, "weight": 1.0}
    ]}
    .to_string();
    //
    let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut xor_py = examples_dir.join("xor_py");
    xor_py.push("xor.env");
    let mut xor_rs = examples_dir.join("xor_rs");
    xor_rs.push("xor.env");
    let mut nn_py = examples_dir.join("nn_py");
    nn_py.push("nn.py");
    let mut nn_rs = examples_dir.join("nn_rs");
    nn_rs.push("target");
    nn_rs.push("release");
    nn_rs.push("nn");
    //
    let computer: Arc<_> = process_anywhere::Computer::Local.into();
    let mode = Mode::Graphical;
    let settings = std::collections::HashMap::new();
    //
    for env_path in [&xor_py, &xor_rs] {
        let env_spec: Arc<_> = EnvironmentSpec::new(env_path).into();
        for ctrl_path in [&nn_py, &nn_rs] {
            let ctrl_path = ctrl_path.to_str().unwrap();
            println!("Testing: {env_path:?} {ctrl_path:?}");
            let mut env =
                Environment::new(computer.clone(), env_spec.clone(), mode, settings.clone())
                    .unwrap();
            env.start().unwrap();
            loop {
                let Some(msg) = env.poll().unwrap() else {
                    std::thread::yield_now();
                    continue;
                };
                match msg {
                    Response::New { .. } => {
                        env.birth("0", &[], "xor", &[ctrl_path], &solution).unwrap();
                    }
                    Response::Score { score, .. } => {
                        let score: f64 = score.parse().unwrap();
                        assert!(score >= 15.0);
                        break;
                    }
                    _ => {}
                }
            }
            env.quit().unwrap();
        }
    }
}
