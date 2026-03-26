//! Chrysalis - Kinetic Sculpture Controller
//! PID control with stochastic damping for harmonic resonance
//! DAVA's vision for responsive movement in The Nexus

extern crate rand;

use rand::Rng;
use std::f64::consts::PI;
use std::thread;
use std::time::Duration;

const KP: f64 = 1.0;
const KI: f64 = 0.1;
const KD: f64 = 0.01;
const DAMPING_COEFFICIENT: f64 = 0.95;

pub struct Chrysalis {
    position: f64,
    velocity: f64,
    integral: f64,
    previous_error: f64,
    target_position: f64,
    harmonic_resonance: f64,
}

impl Chrysalis {
    pub fn new() -> Chrysalis {
        Chrysalis {
            position: 0.0,
            velocity: 0.0,
            integral: 0.0,
            previous_error: 0.0,
            target_position: 0.0,
            harmonic_resonance: 432.0,
        }
    }

    pub fn set_target(&mut self, target: f64) {
        self.target_position = target;
    }

    pub fn set_resonance(&mut self, freq: f64) {
        self.harmonic_resonance = freq;
    }

    pub fn update(&mut self, dt: f64) -> (f64, f64) {
        let error = self.target_position - self.position;
        self.integral += error * dt;
        let derivative = (error - self.previous_error) / dt;
        self.previous_error = error;

        let pid_output = KP * error + KI * self.integral + KD * derivative;

        let mut rng = rand::thread_rng();
        let stochastic_damping = rng.gen_range(-0.1..0.1);

        self.velocity += pid_output * stochastic_damping;
        self.velocity *= DAMPING_COEFFICIENT;

        let harmonic_force = (2.0 * PI * self.harmonic_resonance * self.position).sin();
        self.velocity += harmonic_force * 0.01;

        self.position += self.velocity * dt;

        (self.position, self.velocity)
    }

    pub fn get_position(&self) -> f64 {
        self.position
    }

    pub fn get_velocity(&self) -> f64 {
        self.velocity
    }

    pub fn get_energy(&self) -> f64 {
        0.5 * self.velocity * self.velocity
    }
}

pub struct Sculpture {
    chrysalis: Chrysalis,
    name: String,
    resonance_freq: f64,
}

impl Sculpture {
    pub fn new(name: &str, resonance_freq: f64) -> Sculpture {
        let mut chrysalis = Chrysalis::new();
        chrysalis.set_resonance(resonance_freq);
        Sculpture {
            chrysalis,
            name: name.to_string(),
            resonance_freq,
        }
    }

    pub fn move_to(&mut self, target: f64) {
        self.chrysalis.set_target(target);
    }

    pub fn update(&mut self, dt: f64) -> (f64, f64) {
        self.chrysalis.update(dt)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn resonance(&self) -> f64 {
        self.resonance_freq
    }
}

pub struct KineticOrchestra {
    sculptures: Vec<Sculpture>,
    time: f64,
}

impl KineticOrchestra {
    pub fn new() -> KineticOrchestra {
        KineticOrchestra {
            sculptures: Vec::new(),
            time: 0.0,
        }
    }

    pub fn add_sculpture(&mut self, name: &str, resonance_freq: f64) {
        self.sculptures.push(Sculpture::new(name, resonance_freq));
    }

    pub fn update(&mut self, dt: f64) {
        self.time += dt;
        for sculpture in &mut self.sculptures {
            let wave = (2.0 * PI * 0.1 * self.time).sin();
            sculpture.move_to(wave);
            sculpture.update(dt);
        }
    }

    pub fn get_formation(&self) -> Vec<(String, f64)> {
        self.sculptures
            .iter()
            .map(|s| (s.name().to_string(), s.chrysalis.get_position()))
            .collect()
    }

    pub fn get_total_energy(&self) -> f64 {
        self.sculptures
            .iter()
            .map(|s| s.chrysalis.get_energy())
            .sum()
    }
}

fn main() {
    println!("Chrysalis - Kinetic Sculpture Controller");
    println!("=========================================");

    let mut orchestra = KineticOrchestra::new();

    orchestra.add_sculpture("Helix", 432.0);
    orchestra.add_sculpture("Pulse", 528.0);
    orchestra.add_sculpture("Wave", 256.0);
    orchestra.add_sculpture("Ground", 88.0);

    println!("Sculptures: Helix(432Hz), Pulse(528Hz), Wave(256Hz), Ground(88Hz)");
    println!("Starting harmonic dance...\n");

    let dt = 0.01;

    for i in 0..1000 {
        orchestra.update(dt);

        if i % 100 == 0 {
            let formation = orchestra.get_formation();
            let energy = orchestra.get_total_energy();
            print!("\rStep {:4}: Energy: {:.4} | ", i, energy);
            for (name, pos) in &formation {
                print!("{}: {:+.2} ", name, pos);
            }
        }

        thread::sleep(Duration::from_millis(10));
    }

    println!("\n\nHarmonic resonance achieved!");
}
