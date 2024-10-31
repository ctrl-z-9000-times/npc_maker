fn xor_test(ctrl: &mut ctrl_api::Instance, verbose: bool) -> f64 {
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
    let (env_spec, mode, _settings) = env_api::get_args();

    let mut ctrl: Option<ctrl_api::Instance> = None;

    loop {
        let Some(request) = env_api::poll().unwrap() else {
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        };
        match request {
            env_api::Request::Quit => break,

            env_api::Request::Heartbeat | env_api::Request::Stop | env_api::Request::Pause => {
                env_api::ack(&request).unwrap()
            }

            env_api::Request::Save(_) | env_api::Request::Load(_) => {
                // Save/Load are unimplemented for this environment, do nothing.
            }

            env_api::Request::Start | env_api::Request::Resume => {
                env_api::ack(&request).unwrap();
                env_api::request_new(None).unwrap();
            }

            env_api::Request::Birth {
                individual,
                population,
                controller,
                genotype,
            } => {
                assert_eq!(population, "xor");
                if let Some(ctrl) = &ctrl {
                    assert_eq!(ctrl.get_command(), controller);
                } else {
                    ctrl = Some(ctrl_api::Instance::new(&env_spec.spec, &population, &controller).unwrap());
                }
                let ctrl = ctrl.as_mut().unwrap();

                let genotype = serde_json::to_string(&genotype).unwrap();
                ctrl.new_genotype(&genotype).unwrap();
                let score = xor_test(ctrl, mode == env_api::Mode::Graphical);
                env_api::report_score(Some(&population), Some(individual), score).unwrap();
                env_api::report_death(Some(&population), Some(individual)).unwrap();
                env_api::request_new(Some(&population)).unwrap();
            }
        }
    }

    if let Some(ctrl) = &mut ctrl {
        ctrl.quit().unwrap();
    }
}
