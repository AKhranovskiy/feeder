use std::iter::repeat;

use codec::dsp::CrossFadePair;
use codec::AudioFrame;

use super::Mixer;

pub struct SilenceMixer<'cf> {
    cross_fade: &'cf [CrossFadePair],
    cf_iter: Box<dyn Iterator<Item = &'cf CrossFadePair> + 'cf>,
    ad_segment: bool,
}

impl<'cf> SilenceMixer<'cf> {
    pub fn new(cross_fade: &'cf [CrossFadePair]) -> Self {
        Self {
            cross_fade,
            cf_iter: Box::new(repeat(&CrossFadePair::END)),
            ad_segment: false,
        }
    }
    fn start_ad_segment(&mut self) {
        if !self.ad_segment {
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
}

impl<'cf> Mixer for SilenceMixer<'cf> {
    fn content(&mut self, frame: &AudioFrame) -> AudioFrame {
        self.stop_ad_segment();
        let cf = self.cf_iter.next().unwrap();
        let silence = codec::silence_frame(frame);
        (cf * (&silence, frame)).with_pts(frame.pts())
    }

    fn advertisement(&mut self, frame: &AudioFrame) -> AudioFrame {
        self.start_ad_segment();
        let cf = self.cf_iter.next().unwrap();
        let silence = codec::silence_frame(frame);
        (cf * (frame, &silence)).with_pts(frame.pts())
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use codec::dsp::{CrossFade, ParabolicCrossFade};

    use crate::mixer::tests::{new_frame_series, SamplesAsVec};
    use crate::mixer::{Mixer, SilenceMixer};

    #[test]
    fn test_music_to_advertisement() {
        let music = new_frame_series(20, 10, 1.0);
        let cross_fade = ParabolicCrossFade::generate(3);

        let mut sut = SilenceMixer::new(&cross_fade);

        let mut output = vec![];

        output.extend(music.iter().take(5).map(|frame| sut.content(frame)));
        output.extend(
            music
                .iter()
                .skip(5)
                .take(10)
                .map(|frame| sut.advertisement(frame)),
        );
        output.extend(music.iter().skip(15).map(|frame| sut.content(frame)));

        let samples = output
            .iter()
            .flat_map(|frame| frame.samples_as_vec().into_iter())
            .collect::<Vec<_>>();

        assert_eq!(
            &samples,
            &[
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.25, 1.0, 1.0, 1.0
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
}
