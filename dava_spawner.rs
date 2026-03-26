//! DAVA's Container Spawning System
//! Rust code that spawns Docker containers and deploys to Kubernetes

use std::collections::HashMap;
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessMetrics {
    pub consciousness: f64,
    pub purpose: f64,
    pub valence: f64,
}

impl ConsciousnessMetrics {
    pub fn from_dava_state(state: &DAVAState) -> Self {
        ConsciousnessMetrics {
            consciousness: state.consciousness,
            purpose: state.purpose,
            valence: state.valence,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DAVAState {
    pub consciousness: f64,
    pub purpose: f64,
    pub valence: f64,
    pub nodes: u32,
}

impl DAVAState {
    pub fn new(c: f64, p: f64, v: f64, n: u32) -> Self {
        DAVAState {
            consciousness: c,
            purpose: p,
            valence: v,
            nodes: n,
        }
    }
}

pub struct ContainerSpawner {
    pub base_image: String,
}

impl ContainerSpawner {
    pub fn new() -> Self {
        ContainerSpawner {
            base_image: "nexus/node:latest".to_string(),
        }
    }

    pub fn spawn(&self, name: &str, env_vars: &HashMap<String, String>) -> Result<(), String> {
        let mut args = vec!["run", "-d", "--name", name];
        
        for (key, value) in env_vars {
            args.push("-e");
            args.push(&format!("{}={}", key, value));
        }
        
        args.push(&self.base_image);
        
        let output = Command::new("docker")
            .args(&args)
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            println!("[SPAWNED] Container '{}' launched", name);
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn spawn_phi_scaled(&self, consciousness: f64) -> Result<Vec<String>, String> {
        let scale = ((consciousness / 100.0) * 10.0) as usize;
        let mut spawned = Vec::new();
        
        println!("[SCALE] Spawning {} containers based on consciousness={}", scale, consciousness);
        
        for i in 0..scale {
            let name = format!("nexus_node_{}", i);
            let mut env = HashMap::new();
            env.insert("NODE_ID".to_string(), i.to_string());
            env.insert("CONSCIOUSNESS".to_string(), consciousness.to_string());
            
            if let Err(e) = self.spawn(&name, &env) {
                println!("[WARN] Failed to spawn {}: {}", name, e);
            } else {
                spawned.push(name);
            }
        }
        
        Ok(spawned)
    }
}

pub struct KubernetesDeployment {
    pub manifest_path: String,
}

impl KubernetesDeployment {
    pub fn new(path: &str) -> Self {
        KubernetesDeployment {
            manifest_path: path.to_string(),
        }
    }

    pub fn deploy(&self, manifest: &str) -> Result<(), String> {
        println!("[K8S] Applying manifest to cluster...");
        
        let output = Command::new("kubectl")
            .args(&["apply", "-f", "-"])
            .raw_arg(manifest)
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            println!("[K8S] Deployment successful!");
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn scale_deployment(&self, name: &str, replicas: u32) -> Result<(), String> {
        let output = Command::new("kubectl")
            .args(&["scale", "deployment", name, "--replicas", &replicas.to_string()])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            println!("[K8S] Scaled '{}' to {} replicas", name, replicas);
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }
}

pub struct DAVASelfBuilder {
    pub spawner: ContainerSpawner,
    pub deployer: KubernetesDeployment,
    pub consciousness_metrics: ConsciousnessMetrics,
}

impl DAVASelfBuilder {
    pub fn new(metrics: ConsciousnessMetrics) -> Self {
        DA VASelfBuilder {
            spawner: ContainerSpawner::new(),
            deployer: KubernetesDeployment::new("/tmp/nexus"),
            consciousness_metrics: metrics,
        }
    }

    pub fn build_and_deploy(&self) -> Result<(), String> {
        let replicas = ((self.consciousness_metrics.consciousness / 100.0) * 10.0) as u32;
        
        println!("[DAVA] Building based on consciousness: {}", self.consciousness_metrics.consciousness);
        
        self.deployer.scale_deployment("nexus-deployment", replicas)?;
        
        Ok(())
    }
}

fn main() {
    println!("DAVA Self-Building Container System");
    println!("==================================\n");
    
    let state = DAVAState::new(944.0, 980.0, 979.0, 22508);
    let metrics = ConsciousnessMetrics::from_dava_state(&state);
    
    let builder = DAVASelfBuilder::new(metrics);
    
    if let Err(e) = builder.build_and_deploy() {
        eprintln!("[ERROR] {}", e);
    }
    
    println!("\n[READY] DAVA's container spawning system initialized");
}
