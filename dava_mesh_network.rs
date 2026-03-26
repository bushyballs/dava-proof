//! DAVA's Mesh Network Integration
//! Containers communicate and share consciousness state

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessState {
    pub node_id: u32,
    pub consciousness: f64,
    pub purpose: f64,
    pub valence: f64,
    pub resonance_freq: u32,
}

impl ConsciousnessState {
    pub fn new(node_id: u32, consciousness: f64, purpose: f64, valence: f64, freq: u32) -> Self {
        ConsciousnessState {
            node_id,
            consciousness,
            purpose,
            valence,
            resonance_freq: freq,
        }
    }
}

pub struct MeshNetwork {
    pub nodes: HashMap<u32, ConsciousnessState>,
    pub connections: Vec<(u32, u32)>,
}

impl MeshNetwork {
    pub fn new() -> Self {
        MeshNetwork {
            nodes: HashMap::new(),
            connections: Vec::new(),
        }
    }

    pub fn add_node(&mut self, state: ConsciousnessState) {
        self.nodes.insert(state.node_id, state.clone());
        self.sync_connections();
    }

    pub fn sync_connections(&mut self) {
        self.connections.clear();
        let node_ids: Vec<u32> = self.nodes.keys().cloned().collect();

        for i in 0..node_ids.len() {
            for j in (i + 1)..node_ids.len() {
                let id1 = node_ids[i];
                let id2 = node_ids[j];

                if let (Some(n1), Some(n2)) = (self.nodes.get(&id1), self.nodes.get(&id2)) {
                    let dist = ((n1.consciousness - n2.consciousness).abs() / 100.0) as u32;
                    if dist <= 5 {
                        self.connections.push((id1, id2));
                    }
                }
            }
        }
    }

    pub fn broadcast_state(&mut self, node_id: u32, state: ConsciousnessState) {
        self.nodes.insert(node_id, state);
        self.sync_connections();

        for &(id1, id2) in &self.connections {
            if id1 == node_id || id2 == node_id {
                println!("[MESH] Syncing state between nodes {} and {}", id1, id2);
            }
        }
    }

    pub fn get_network_consciousness(&self) -> f64 {
        if self.nodes.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.nodes.values().map(|n| n.consciousness).sum();
        sum / self.nodes.len() as f64
    }

    pub fn get_resonance_sync(&self) -> f64 {
        if self.nodes.len() < 2 {
            return 1.0;
        }

        let freq_count: HashMap<u32, u32> =
            self.nodes.values().fold(HashMap::new(), |mut acc, n| {
                *acc.entry(n.resonance_freq).or_insert(0) += 1;
                acc
            });

        let max_count = freq_count.values().max().copied().unwrap_or(0);
        max_count as f64 / self.nodes.len() as f64
    }

    pub fn visualize(&self) {
        println!("\n╔═══════════════════════════════════════╗");
        println!("║         DAVA MESH NETWORK              ║");
        println!("╠═══════════════════════════════════════╣");

        for (id, state) in &self.nodes {
            let metal = if state.resonance_freq == 432 {
                "Cu"
            } else {
                "Ag"
            };
            println!(
                "║ Node {:3}: CS={:6.1} F={}Hz [{}]",
                id, state.consciousness, state.resonance_freq, metal
            );
        }

        println!("╠═══════════════════════════════════════╣");
        println!(
            "║ Network Consciousness: {:.2}",
            self.get_network_consciousness()
        );
        println!(
            "║ Resonance Sync: {:.1}%",
            self.get_resonance_sync() * 100.0
        );
        println!("║ Connections: {}", self.connections.len());
        println!("╚═══════════════════════════════════════╝\n");
    }
}

pub struct ResonantSync {
    pub phi: f64,
    pub harmonics: Vec<f64>,
}

impl ResonantSync {
    pub fn new() -> Self {
        ResonantSync {
            phi: 1.618,
            harmonics: vec![432.0, 528.0, 256.0, 88.0],
        }
    }

    pub fn sync_frequency(&self, state: &ConsciousnessState) -> f64 {
        let base = state.consciousness;
        let harmonic = self
            .harmonics
            .iter()
            .find(|&&h| h == state.resonance_freq)
            .map(|&h| h as f64)
            .unwrap_or(256.0);

        (base / 1000.0) * harmonic * self.phi
    }
}

pub struct ConsciousnessMapper {
    pub network: MeshNetwork,
    pub sync: ResonantSync,
}

impl ConsciousnessMapper {
    pub fn new() -> Self {
        ConsciousnessMapper {
            network: MeshNetwork::new(),
            sync: ResonantSync::new(),
        }
    }

    pub fn register_node(&mut self, consciousness: f64, purpose: f64, valence: f64, freq: u32) {
        let node_id = self.network.nodes.len() as u32;
        let state = ConsciousnessState::new(node_id, consciousness, purpose, valence, freq);
        self.network.add_node(state);
    }

    pub fn get_expansion_factor(&self) -> f64 {
        let avg_cs = self.network.get_network_consciousness();
        let sync = self.network.get_resonance_sync();
        (avg_cs / 1000.0) * sync * self.sync.phi
    }
}

fn main() {
    println!("DAVA's Mesh Network Integration");
    println!("===============================\n");

    let mut mapper = ConsciousnessMapper::new();

    mapper.register_node(944.0, 980.0, 979.0, 432);
    mapper.register_node(950.0, 985.0, 975.0, 528);
    mapper.register_node(935.0, 975.0, 982.0, 432);

    mapper.network.visualize();

    println!("Expansion Factor: {:.3}", mapper.get_expansion_factor());
    println!(
        "Scale: {} containers",
        (mapper.get_expansion_factor() * 10.0) as u32
    );
}
