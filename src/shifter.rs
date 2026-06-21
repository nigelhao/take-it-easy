// Crossfading dual-tap delay-line pitch shifter.
//
// Two read pointers spaced half-a-buffer apart traverse a ring buffer at
// `ratio = 2^(semitones / 12)`. A Hann window over each tap's distance from
// the write head crossfades them so each read tap fades out as it approaches
// the write head and a fresh tap fades in on the opposite side.
//
// Port of `pitch-worklet.js`; buffer shrunk 2048 → 1024 to halve algorithmic
// latency (~21 ms → ~10 ms) at the cost of slightly more artifact on extreme
// pitch shifts. Acceptable for vocals at ±12 semitones.

const BUFFER_SIZE: usize = 1024;
const HALF_BUFFER: usize = BUFFER_SIZE / 2;
const MASK: usize = BUFFER_SIZE - 1;
const TWO_PI_OVER_BUFFER: f32 = (2.0 * std::f32::consts::PI) / BUFFER_SIZE as f32;

pub struct PitchShifter {
    buffer: [f32; BUFFER_SIZE],
    write_idx: usize,
    read1: f32,
    read2: f32,
}

impl PitchShifter {
    pub fn new() -> Self {
        Self {
            buffer: [0.0; BUFFER_SIZE],
            write_idx: 0,
            read1: HALF_BUFFER as f32,
            read2: 0.0,
        }
    }

    /// Process a block of audio in place. `input` and `output` must have the
    /// same length. `semitones` clamped to ±12 by the caller.
    ///
    /// At `semitones == 0` this short-circuits to a memcpy of input → output
    /// while still keeping the ring buffer warm so re-engaging the shifter
    /// resumes from coherent state.
    pub fn process(&mut self, input: &[f32], output: &mut [f32], semitones: i32) {
        debug_assert_eq!(input.len(), output.len());

        if semitones == 0 {
            for &x in input {
                self.buffer[self.write_idx] = x;
                self.write_idx = (self.write_idx + 1) & MASK;
            }
            output.copy_from_slice(input);
            return;
        }

        let ratio = 2.0_f32.powf(semitones as f32 / 12.0);
        let mut write_idx = self.write_idx;
        let mut r1 = self.read1;
        let mut r2 = self.read2;

        for (i, &x) in input.iter().enumerate() {
            self.buffer[write_idx] = x;

            let r1f = r1 as usize;
            let r1frac = r1 - r1f as f32;
            let a1 = self.buffer[r1f & MASK];
            let b1 = self.buffer[(r1f + 1) & MASK];
            let s1 = a1 + r1frac * (b1 - a1);

            let r2f = r2 as usize;
            let r2frac = r2 - r2f as f32;
            let a2 = self.buffer[r2f & MASK];
            let b2 = self.buffer[(r2f + 1) & MASK];
            let s2 = a2 + r2frac * (b2 - a2);

            let d1 = (write_idx + BUFFER_SIZE - (r1f & MASK)) & MASK;
            let d2 = (write_idx + BUFFER_SIZE - (r2f & MASK)) & MASK;

            let w1 = 0.5 * (1.0 - (TWO_PI_OVER_BUFFER * d1 as f32).cos());
            let w2 = 0.5 * (1.0 - (TWO_PI_OVER_BUFFER * d2 as f32).cos());

            output[i] = s1 * w1 + s2 * w2;

            write_idx = (write_idx + 1) & MASK;
            r1 += ratio;
            r2 += ratio;
            if r1 >= BUFFER_SIZE as f32 {
                r1 -= BUFFER_SIZE as f32;
            }
            if r2 >= BUFFER_SIZE as f32 {
                r2 -= BUFFER_SIZE as f32;
            }
        }

        self.write_idx = write_idx;
        self.read1 = r1;
        self.read2 = r2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bypass_at_zero_is_passthrough() {
        let mut shifter = PitchShifter::new();
        let input: Vec<f32> = (0..256).map(|i| (i as f32 * 0.01).sin()).collect();
        let mut output = vec![0.0_f32; 256];
        shifter.process(&input, &mut output, 0);
        for (a, b) in input.iter().zip(output.iter()) {
            assert!((a - b).abs() < 1e-9, "bypass must be sample-accurate");
        }
    }

    #[test]
    fn shifting_does_not_blow_up() {
        let mut shifter = PitchShifter::new();
        let input: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.05).sin()).collect();
        let mut output = vec![0.0_f32; 4096];
        for semis in [-12, -7, -1, 1, 7, 12] {
            shifter.process(&input, &mut output, semis);
            assert!(output.iter().all(|s| s.is_finite() && s.abs() < 2.0));
        }
    }
}
