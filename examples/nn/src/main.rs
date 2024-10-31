//! Example controller - artificial neural network
//!
//! This file demonstrates how to implement a control system for the NPC Maker.

use ctrl_api::{poll, Message};
use serde::Deserialize;
use std::collections::HashMap;
use std::io;

pub fn logistic(value: f64, slope: f64, midpoint: f64) -> f64 {
    // The magic number 4.0 scales the maximum slope of the curve to 1.0
    let x = 4.0 * slope * (value - midpoint);
    1.0 / (1.0 + (-x).exp())
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum Chromosome {
    Node {
        name: u64,
        midpoint: f64,
        slope: f64,
    },

    Edge {
        #[serde(default)]
        name: u64,
        presyn: u64,
        postsyn: u64,
        weight: f64,
    },
}

impl Chromosome {
    pub fn name(&self) -> u64 {
        match self {
            Self::Node { name, .. } => *name,
            Self::Edge { name, .. } => *name,
        }
    }
}

#[derive(Debug, Default)]
struct NeuralNetwork {
    /// Maps name to index
    names: HashMap<u64, u64>,

    nodes: Vec<(f64, f64)>,

    state: Vec<f64>,

    edges: Vec<(u64, u64, f64)>,
}

impl NeuralNetwork {
    pub fn new(&mut self, genotype: &str) {
        //
        let mut genotype: Vec<Chromosome> = serde_json::from_str(genotype).unwrap();
        genotype.sort_unstable_by_key(|x| x.name());
        //
        self.names = genotype
            .iter()
            .enumerate()
            .filter_map(|(idx, chrom)| match chrom {
                Chromosome::Node { .. } => Some((chrom.name(), idx as u64)),
                Chromosome::Edge { .. } => None,
            })
            .collect();
        //
        self.nodes = genotype
            .iter()
            .filter_map(|chrom| match chrom {
                Chromosome::Node {
                    slope, midpoint, ..
                } => Some((*slope, *midpoint)),
                Chromosome::Edge { .. } => None,
            })
            .collect();
        //
        self.state = vec![0.0; self.nodes.len()];
        //
        self.edges = genotype
            .iter()
            .filter_map(|chrom| match chrom {
                Chromosome::Node { .. } => None,
                Chromosome::Edge {
                    presyn,
                    postsyn,
                    weight,
                    ..
                } => Some((self.names[presyn], self.names[postsyn], *weight)),
            })
            .collect();
    }

    pub fn reset(&mut self) {
        self.state.fill(0.0);
    }

    pub fn advance(&mut self, _dt: f64) {
        let mut next_state = vec![0.0; self.nodes.len()];
        for (presyn, postsyn, weight) in self.edges.iter().copied() {
            next_state[postsyn as usize] += weight * self.state[presyn as usize];
        }
        for ((slope, midpoint), value) in self.nodes.iter().copied().zip(&mut next_state) {
            *value = logistic(*value, slope, midpoint);
        }
        self.state = next_state;
    }
}

fn main() -> Result<(), io::Error> {
    let mut nn = NeuralNetwork::default();
    loop {
        let message = poll()?;
        match message {
            Message::New { genotype } => {
                nn.new(&genotype);
            }
            Message::Reset => {
                nn.reset();
            }
            Message::SetInput { gin, value } => {
                nn.state[gin as usize] = value.parse().unwrap();
            }
            Message::GetOutput { gin } => {
                ctrl_api::send_output(gin, nn.state[gin as usize].to_string())?;
            }
            Message::Advance { dt } => {
                nn.advance(dt);
            }

            Message::Quit => break,

            Message::Environment { .. } | Message::Population { .. } => {}

            _ => panic!("unsupported operation: {message:?}"),
        }
    }
    Ok(())
}
