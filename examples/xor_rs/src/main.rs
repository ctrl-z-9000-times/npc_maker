use npc_maker::ctrl::Controller;
use npc_maker::env;
use npc_maker::env::Request;

fn xor_test(ctrl: &mut Controller, verbose: bool) -> f64 {
    let mut distance: f64 = 0.0;
    for input_1 in 0..=1 {
        for input_2 in 0..=1 {
            ctrl.reset().unwrap();
            let mut prev = None;
            let mut steadystate = false;
            for _ in 0..4 {
                ctrl.set_input(0, &format!("{input_1}")).unwrap();
                ctrl.set_input(1, &format!("{input_2}")).unwrap();
                ctrl.advance(1.0).unwrap();
                let output: f64 = ctrl.get_outputs(&[2]).unwrap()[&2].parse().unwrap();
                if Some(output) == prev {
                    if verbose {
                        eprintln!("{input_1} xor {input_2} = {output}")
                    };
                    let correct: f64 = (input_1 != input_2) as i64 as f64;
                    distance += (correct - output).abs();
                    steadystate = true;
                    break;
                }
                prev = Some(output);
            }
            if !steadystate {
                return f64::NAN;
            }
        }
    }
    let score = (4.0 - distance).powi(2);
    if verbose {
        eprintln!("score {score}")
    };
    score
}

fn main() {
    let (env_spec, mode, _settings) = npc_maker::env::get_args();

    let mut ctrl: Option<Controller> = None;

    loop {
        let Some(request) = npc_maker::env::poll().unwrap() else {
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        };

        match request {
            Request::Quit => break,

            Request::Heartbeat | Request::Stop | Request::Pause => env::ack(&request),

            Request::Save(_) | Request::Load(_) | Request::Custom(_) => {
                // Save/Load are unimplemented for this environment, do nothing.
            }

            Request::Start | Request::Resume => {
                env::ack(&request);
                env::new(Some("xor"));
            }

            Request::Birth {
                name,
                controller,
                genome,
                ..
            } => {
                if let Some(ctrl) = &ctrl {
                    assert_eq!(ctrl.get_command(), controller);
                } else {
                    ctrl = Some(Controller::new(&env_spec.spec, "xor", controller).unwrap());
                }
                let ctrl = ctrl.as_mut().unwrap();

                let genome = serde_json::to_string(&genome).unwrap();
                ctrl.genome(&genome).unwrap();
                let score = xor_test(ctrl, mode == env::Mode::Graphical);
                env::score(Some(&name), &score.to_string());
                env::death(Some(&name));
                env::new(Some("xor"));
            }
        }
    }

    if let Some(ctrl) = &mut ctrl {
        ctrl.quit().unwrap();
    }
}
