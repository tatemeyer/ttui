use crate::buffer::{Buffer, Cell};
use crossterm::style::Color;
use std::time::Duration;

pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub symbol: char,
    pub color: Color,
    pub lifetime: Duration,
    pub age: Duration,
}

impl Particle {
    pub fn is_alive(&self) -> bool {
        self.age < self.lifetime
    }
}

#[derive(Default)]
pub struct ParticleSystem {
    particles: Vec<Particle>,
}

impl ParticleSystem {
    pub fn new() -> Self {
        ParticleSystem::default()
    }

    pub fn spawn(&mut self, p: Particle) {
        self.particles.push(p);
    }

    pub fn update(&mut self, elapsed: Duration) {
        let dt = elapsed.as_secs_f32();
        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.age += elapsed;
        }
        self.particles.retain(|p| p.is_alive());
    }

    pub fn render(&self, buf: &mut Buffer) {
        for p in &self.particles {
            let x = p.x.round();
            let y = p.y.round();
            if x >= 0.0 && y >= 0.0 && (x as u16) < buf.width && (y as u16) < buf.height {
                #[allow(clippy::needless_update)]
                // Keep ..Default::default() even though it's a no-op today (all 3 fields are
                // already set). A sibling task is concurrently adding a 4th `style` field to
                // Cell; this syntax ensures the file compiles unchanged once that field lands.
                let cell = Cell {
                    symbol: p.symbol,
                    fg: p.color,
                    bg: Color::Reset,
                    ..Default::default()
                };
                buf.set(x as u16, y as u16, cell);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_system_new_is_empty() {
        let ps = ParticleSystem::new();
        assert_eq!(ps.len(), 0);
        assert!(ps.is_empty());
    }

    #[test]
    fn spawn_increases_len_by_one() {
        let mut ps = ParticleSystem::new();
        let p = Particle {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            symbol: '*',
            color: Color::Red,
            lifetime: Duration::from_millis(100),
            age: Duration::ZERO,
        };
        ps.spawn(p);
        assert_eq!(ps.len(), 1);
    }

    #[test]
    fn update_moves_particle_by_velocity_times_elapsed() {
        let mut ps = ParticleSystem::new();
        let p = Particle {
            x: 0.0,
            y: 0.0,
            vx: 2.0,
            vy: 0.0,
            symbol: '*',
            color: Color::Red,
            lifetime: Duration::from_millis(1000),
            age: Duration::ZERO,
        };
        ps.spawn(p);
        ps.update(Duration::from_millis(500));
        // With vx=2.0 and elapsed=500ms=0.5s, x should advance by 2.0 * 0.5 = 1.0
        // We need to check through render or by accessing particles directly
        // Since particles is private, we'll check via rendering behavior
        let mut buf = Buffer::new(10, 10);
        ps.render(&mut buf);
        // The particle should be at x=1.0 (rounded to 1), y=0.0
        assert_eq!(
            *buf.get(1, 0),
            Cell {
                symbol: '*',
                fg: Color::Red,
                bg: Color::Reset,
            }
        );
    }

    #[test]
    fn update_ages_particle_and_removes_when_age_exceeds_lifetime() {
        let mut ps = ParticleSystem::new();
        let p = Particle {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            symbol: '*',
            color: Color::Red,
            lifetime: Duration::from_millis(100),
            age: Duration::ZERO,
        };
        ps.spawn(p);
        assert_eq!(ps.len(), 1);
        ps.update(Duration::from_millis(150));
        assert_eq!(ps.len(), 0);
    }

    #[test]
    fn update_retains_particles_with_age_less_than_lifetime() {
        let mut ps = ParticleSystem::new();
        let p = Particle {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            symbol: '*',
            color: Color::Red,
            lifetime: Duration::from_millis(100),
            age: Duration::ZERO,
        };
        ps.spawn(p);
        assert_eq!(ps.len(), 1);
        ps.update(Duration::from_millis(50));
        assert_eq!(ps.len(), 1);
    }

    #[test]
    fn render_writes_particle_to_buffer_at_rounded_position() {
        let mut ps = ParticleSystem::new();
        let p = Particle {
            x: 3.4,
            y: 2.6,
            vx: 0.0,
            vy: 0.0,
            symbol: 'X',
            color: Color::Blue,
            lifetime: Duration::from_millis(100),
            age: Duration::ZERO,
        };
        ps.spawn(p);
        let mut buf = Buffer::new(10, 10);
        ps.render(&mut buf);
        // x=3.4 rounds to 3, y=2.6 rounds to 3
        assert_eq!(
            *buf.get(3, 3),
            Cell {
                symbol: 'X',
                fg: Color::Blue,
                bg: Color::Reset,
            }
        );
    }

    #[test]
    fn render_skips_particle_outside_buffer_bounds() {
        let mut ps = ParticleSystem::new();
        // Particle at negative x
        let p1 = Particle {
            x: -1.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            symbol: 'A',
            color: Color::Red,
            lifetime: Duration::from_millis(100),
            age: Duration::ZERO,
        };
        ps.spawn(p1);
        // Particle at x >= width
        let p2 = Particle {
            x: 10.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            symbol: 'B',
            color: Color::Red,
            lifetime: Duration::from_millis(100),
            age: Duration::ZERO,
        };
        ps.spawn(p2);
        // Particle at negative y
        let p3 = Particle {
            x: 0.0,
            y: -1.0,
            vx: 0.0,
            vy: 0.0,
            symbol: 'C',
            color: Color::Red,
            lifetime: Duration::from_millis(100),
            age: Duration::ZERO,
        };
        ps.spawn(p3);
        // Particle at y >= height
        let p4 = Particle {
            x: 0.0,
            y: 10.0,
            vx: 0.0,
            vy: 0.0,
            symbol: 'D',
            color: Color::Red,
            lifetime: Duration::from_millis(100),
            age: Duration::ZERO,
        };
        ps.spawn(p4);

        let mut buf = Buffer::new(10, 10);
        ps.render(&mut buf);

        // Buffer should still be all default cells
        for y in 0..10 {
            for x in 0..10 {
                assert_eq!(*buf.get(x, y), Cell::default());
            }
        }
    }
}
