use codec::AudioFrame;

mod ads;
mod passthrough;
mod silence;

pub(crate) use ads::AdsMixer;
pub(crate) use passthrough::PassthroughMixer;
pub(crate) use silence::SilenceMixer;

pub trait Mixer {
    fn content(&mut self, frame: &AudioFrame) -> AudioFrame;
    fn advertisement(&mut self, frame: &AudioFrame) -> AudioFrame;
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use ac_ffmpeg::codec::audio::{AudioFrame, AudioFrameMut, ChannelLayout};
    use ac_ffmpeg::time::Timestamp;
    use bytemuck::{cast_slice, cast_slice_mut};
    use codec::SampleFormat;

    pub(super) fn new_frame(pts: Timestamp, content: f32) -> AudioFrame {
        let mut frame = AudioFrameMut::silence(
            ChannelLayout::from_channels(1).unwrap().as_ref(),
            SampleFormat::Flt.into(),
            4,
            4,
        );

        for plane in frame.planes_mut().iter_mut() {
            cast_slice_mut(plane.data_mut())[0] = content;
        }

        frame.freeze().with_pts(pts)
    }

    pub(super) fn new_frame_series(length: usize, start_pts: i64, content: f32) -> Vec<AudioFrame> {
        (0..length)
            .map(|i| new_frame(Timestamp::from_secs(start_pts + i as i64), content))
            .collect()
    }

    #[test]
    fn test_new_frame() {
        let frame = new_frame(Timestamp::from_secs(1), 0.3);
        assert_eq!(frame.samples(), 4);
        assert_eq!(frame.pts().as_secs().unwrap(), 1);
        assert_eq!(&frame.samples_as_vec(), &[0.3]);
    }

    pub(super) trait SamplesAsVec<T> {
        fn samples_as_vec(&self) -> Vec<T>;
    }

    impl SamplesAsVec<f32> for AudioFrame {
        fn samples_as_vec(&self) -> Vec<f32> {
            let mut samples =
                Vec::with_capacity(self.samples() / 4 * self.channel_layout().channels() as usize);

            for plane in self.planes().iter() {
                samples.extend_from_slice(&cast_slice(plane.data())[..self.samples() / 4]);
            }

            samples
        }
    }
}
