//! Deterministic affect-to-drive modulation. Stateless and bounded.

use super::{Affect, Drive, Drives};

pub fn modulate_drives(drives: Drives, affect: Affect) -> Drives {
    let mut next = drives;
    if affect.valence <= -0.3 {
        scale(&mut next, Drive::Coherence, 1.2);
        scale(&mut next, Drive::Rest, 1.1);
    }
    if affect.arousal >= 0.5 {
        scale(&mut next, Drive::Rest, 1.2);
        scale(&mut next, Drive::Curiosity, 0.9);
    }
    if affect.dominance <= -0.2 {
        scale(&mut next, Drive::Connection, 1.1);
        scale(&mut next, Drive::Mastery, 0.9);
    }
    next
}

fn scale(drives: &mut Drives, drive: Drive, multiplier: f32) {
    let multiplier = multiplier.clamp(0.5, 1.5);
    drives.set(drive, drives.get(drive) * multiplier);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modulation_is_bounded_and_stateless() {
        let drives = Drives::default();
        let affect = Affect {
            valence: -1.0,
            arousal: 1.0,
            dominance: -1.0,
        };
        let once = modulate_drives(drives, affect);
        let twice = modulate_drives(drives, affect);
        assert_eq!(once, twice);
        for drive in Drive::ALL {
            assert!((0.0..=1.0).contains(&once.get(drive)));
        }
    }
}
