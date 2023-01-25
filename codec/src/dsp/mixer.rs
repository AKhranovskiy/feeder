#![allow(dead_code)]

use ac_ffmpeg::codec::audio::AudioFrame;
use bytemuck::{cast_slice, cast_slice_mut};

use super::{CrossFade, CrossFadePair, ParabolicCrossFade};

pub struct Mixer {
    cross_fade: Vec<CrossFadePair>,
    offset: usize,
}

impl Mixer {
    pub fn new(samples: usize) -> Self {
        Self {
            cross_fade: ParabolicCrossFade::generate(samples + 1),
            offset: 0,
        }
    }

    pub fn mix(&mut self, first: &AudioFrame, second: &AudioFrame) -> AudioFrame {
        assert_eq!(
            first.samples(),
            second.samples(),
            "Frames must have equal number of samples, first={}, second={}",
            first.samples(),
            second.samples()
        );

        let samples_per_frame = first.samples();

        let first_planes = first.planes();
        let first_data = cast_slice::<_, f32>(first_planes[0].data());

        let second_planes = second.planes();
        let second_data = cast_slice::<_, f32>(second_planes[0].data());

        let mut frame = first.clone().into_mut();

        let mut planes = frame.planes_mut();
        let data = cast_slice_mut::<_, f32>(planes[0].data_mut());

        eprintln!("MIXED DATA {}", data.len());
        let mut iter = self.cross_fade.iter().skip(self.offset);

        for x in 0..samples_per_frame {
            data[x] = iter
                .next()
                .unwrap_or(&(0.0, 1.0).into())
                .apply(first_data[x], second_data[x]);
        }

        self.offset += samples_per_frame;

        frame.freeze()
    }
}

#[cfg(test)]
mod tests {
    use ac_ffmpeg::codec::audio::AudioFrameMut;

    use crate::CodecParams;

    use super::*;

    // Frame data slice len is 32 multiplier.
    const SAMPLES_PER_FRAME: usize = 32;

    const PARAMS: CodecParams =
        CodecParams::new(SAMPLES_PER_FRAME as u32, crate::SampleFormat::FltPlanar, 1);

    fn test_frame() -> AudioFrame {
        let mut frame = AudioFrameMut::silence(
            &PARAMS.channel_layout(),
            PARAMS.sample_format().into(),
            PARAMS.sample_rate(),
            SAMPLES_PER_FRAME,
        );

        cast_slice_mut::<_, f32>(frame.planes_mut()[0].data_mut()).fill(1.0);

        frame.freeze()
    }

    #[test]
    fn test_mixer() {
        let mixed = Mixer::new(SAMPLES_PER_FRAME - 1).mix(&test_frame(), &test_frame());
        let planes = mixed.planes();
        let samples: &[f32] = cast_slice(planes[0].data());

        assert_eq!(
            samples,
            &[
                1.0, 0.99687827, 0.987513, 0.9719043, 0.950052, 0.9219563, 0.88761705, 0.84703434,
                0.8002081, 0.7471384, 0.6878252, 0.6222685, 0.55046827, 0.47242457, 0.48595214,
                0.49843913, 0.49843913, 0.48595214, 0.47242457, 0.55046827, 0.6222685, 0.6878252,
                0.7471384, 0.8002081, 0.84703434, 0.88761705, 0.9219563, 0.950052, 0.9719043,
                0.987513, 0.99687827, 1.0
            ]
        );
    }
}
