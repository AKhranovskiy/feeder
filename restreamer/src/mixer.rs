use std::iter::repeat;

use analyzer::ContentKind;
use codec::dsp::CrossFadePair;
use codec::AudioFrame;

use crate::play_params::PlayAction;

pub struct Mixer<'a> {
    action: PlayAction,
    ad_frames: &'a [AudioFrame],
    ad_iter: Box<dyn Iterator<Item = &'a AudioFrame> + 'a>,
    cross_fade: &'a [CrossFadePair],
    cf_iter: Box<dyn Iterator<Item = &'a CrossFadePair> + 'a>,
    ad_segment: bool,
}

impl<'a> Mixer<'a> {
    pub fn new(
        action: PlayAction,
        ad_frames: &'a [AudioFrame],
        cross_fade: &'a [CrossFadePair],
    ) -> Self {
        Self {
            action,
            ad_frames,
            ad_iter: Box::new(ad_frames.iter().cycle()),
            cross_fade,
            cf_iter: Box::new(cross_fade.iter().chain(repeat(&CrossFadePair::END))),
            ad_segment: false,
        }
    }

    fn start_ad_segment(&mut self) {
        if !self.ad_segment {
            self.ad_iter = Box::new(self.ad_frames.iter().cycle());
            self.cf_iter = Box::new(self.cross_fade.iter().chain(repeat(&CrossFadePair::END)));
            self.ad_segment = true;
        }
    }

    fn stop_ad_segment(&mut self) {
        if self.ad_segment {
            self.cf_iter = Box::new(self.cross_fade.iter().chain(repeat(&CrossFadePair::END)));
            self.ad_segment = false;
        }
    }

    pub fn push(&mut self, frame: AudioFrame, kind: ContentKind) -> AudioFrame {
        let pts = frame.pts();

        match kind {
            ContentKind::Music | ContentKind::Talk | ContentKind::Unknown => {
                self.stop_ad_segment();

                match self.action {
                    PlayAction::Passthrough => frame,
                    PlayAction::Silence => {
                        let cf = self.cf_iter.next().unwrap();
                        let silence = codec::silence_frame(&frame);
                        cf * (&silence, &frame)
                    }
                    PlayAction::Lang(_) => {
                        let cf = self.cf_iter.next().unwrap();
                        let ad = if cf.fade_out() > 0.0 {
                            self.ad_iter
                                .next()
                                .cloned()
                                .unwrap_or_else(|| codec::silence_frame(&frame))
                        } else {
                            codec::silence_frame(&frame)
                        };
                        cf * (&ad, &frame)
                    }
                }
            }
            ContentKind::Advertisement => {
                self.start_ad_segment();

                match self.action {
                    PlayAction::Passthrough => frame,
                    PlayAction::Silence => {
                        let cf = self.cf_iter.next().unwrap();
                        let silence = codec::silence_frame(&frame);
                        cf * (&frame, &silence)
                    }
                    PlayAction::Lang(_) => {
                        let cf = self.cf_iter.next().unwrap();
                        let ad = if cf.fade_in() > 0.0 {
                            self.ad_iter
                                .next()
                                .cloned()
                                .unwrap_or_else(|| codec::silence_frame(&frame))
                        } else {
                            codec::silence_frame(&frame)
                        };
                        cf * (&frame, &ad)
                    }
                }
            }
        }
        .with_pts(pts)
    }
}

#[cfg(test)]
mod tests {
    use ac_ffmpeg::codec::audio::{AudioFrame, AudioFrameMut, ChannelLayout};
    use ac_ffmpeg::time::Timestamp;
    use analyzer::ContentKind;
    use bytemuck::{cast_slice, cast_slice_mut};
    use codec::dsp::{CrossFade, ParabolicCrossFade};
    use codec::SampleFormat;

    use crate::play_params::PlayAction;

    use super::Mixer;

    fn new_frame(pts: Timestamp, content: f32) -> AudioFrame {
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

    fn new_frame_series(length: usize, start_pts: i64, content: f32) -> Vec<AudioFrame> {
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

    #[test]
    fn test_music_to_advertisement_with_silence() {
        let advertisement = new_frame_series(10, 0, 0.9);
        let music = new_frame_series(20, 10, 0.1);
        let cross_fade = ParabolicCrossFade::generate(3);

        let mut sut = Mixer::new(PlayAction::Silence, &advertisement, &cross_fade);

        let mut output = vec![];

        output.extend(
            music
                .iter()
                .take(5)
                .map(|frame| sut.push(frame.clone(), ContentKind::Music)),
        );
        output.extend(
            music
                .iter()
                .skip(5)
                .take(10)
                .map(|frame| sut.push(frame.clone(), ContentKind::Advertisement)),
        );
        output.extend(
            music
                .iter()
                .skip(15)
                .map(|frame| sut.push(frame.clone(), ContentKind::Music)),
        );

        let samples = output
            .iter()
            .flat_map(|frame| frame.samples_as_vec().into_iter())
            .collect::<Vec<_>>();
        assert_eq!(
            &samples,
            &[
                0.0, 0.025, 0.1, 0.1, 0.1, 0.1, 0.025, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.025, 0.1, 0.1, 0.1
            ]
        );

        let timestamps = output
            .iter()
            .map(|frame| frame.pts().as_secs().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            timestamps,
            &[10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29]
        );
    }

    #[test]
    fn test_music_to_advertisement_with_lang() {
        let advertisement = new_frame_series(10, 0, 0.5);
        let music = new_frame_series(20, 10, 1.0);
        let cross_fade = ParabolicCrossFade::generate(4);

        let mut sut = Mixer::new(PlayAction::Lang("nl".into()), &advertisement, &cross_fade);

        let mut output = vec![];

        output.extend(
            music
                .iter()
                .take(5)
                .map(|frame| sut.push(frame.clone(), ContentKind::Music)),
        );
        output.extend(
            music
                .iter()
                .skip(5)
                .take(10)
                .map(|frame| sut.push(frame.clone(), ContentKind::Advertisement)),
        );
        output.extend(
            music
                .iter()
                .skip(15)
                .map(|frame| sut.push(frame.clone(), ContentKind::Music)),
        );

        let samples = output
            .iter()
            .flat_map(|frame| frame.samples_as_vec().into_iter())
            .collect::<Vec<_>>();
        assert_eq!(
            &samples,
            &[
                0.5, 0.33333334, 0.6666667, 1.0, 1.0, 1.0, 0.6666667, 0.33333334, 0.5, 0.5, 0.5,
                0.5, 0.5, 0.5, 0.5, 0.5, 0.33333334, 0.6666667, 1.0, 1.0
            ]
        );

        let timestamps = output
            .iter()
            .map(|frame| frame.pts().as_secs().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            timestamps,
            &[10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29]
        );
    }

    trait SamplesAsVec<T> {
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
