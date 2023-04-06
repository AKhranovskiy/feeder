use analyzer::ContentKind;
use codec::AudioFrame;

mod ads;
mod ads_provider;
mod passthrough;
mod silence;

pub(crate) use ads::AdsMixer;
pub(crate) use passthrough::PassthroughMixer;
pub(crate) use silence::SilenceMixer;

pub trait Mixer {
    fn push(&mut self, kind: ContentKind, frame: AudioFrame) -> AudioFrame;
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use ac_ffmpeg::codec::audio::{AudioFrame, AudioFrameMut, ChannelLayout};
    use ac_ffmpeg::time::Timestamp;
    use bytemuck::{cast_slice, cast_slice_mut};
    use codec::{Pts, SampleFormat};

    fn empty_frame() -> AudioFrame {
        AudioFrameMut::silence(
            ChannelLayout::from_channels(1).unwrap().as_ref(),
            SampleFormat::Flt.into(),
            4,
            4,
        )
        .freeze()
    }

    fn frame_with_content(content: f32) -> AudioFrame {
        let mut frame = empty_frame().into_mut();

        for plane in frame.planes_mut().iter_mut() {
            cast_slice_mut(plane.data_mut())[0] = content;
        }

        frame.freeze()
    }

    pub(super) fn create_frames(length: usize, content: f32) -> Vec<AudioFrame> {
        let mut pts = Pts::from(&empty_frame());
        (0..length)
            .map(|_| frame_with_content(content).with_pts(pts.next()))
            .collect()
    }

    pub(super) fn pts_seq(length: usize) -> Vec<Timestamp> {
        let frame = AudioFrameMut::silence(
            ChannelLayout::from_channels(1).unwrap().as_ref(),
            SampleFormat::Flt.into(),
            4,
            4,
        )
        .freeze();

        let mut pts = Pts::from(&frame);

        (0..length).map(|_| pts.next()).collect()
    }

    // TODO add macro verify_pts
    // TOOD add macro verify_frame_content

    #[test]
    fn test_new_frame() {
        let frame = frame_with_content(0.3);
        assert_eq!(frame.samples(), 4);
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
