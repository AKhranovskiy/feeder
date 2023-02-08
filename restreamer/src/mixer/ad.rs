use std::iter::repeat;

use codec::dsp::CrossFadePair;
use codec::AudioFrame;

use super::Mixer;

pub struct AdMixer<'af, 'cf> {
    ad_frames: &'af [AudioFrame],
    ad_iter: Box<dyn Iterator<Item = &'af AudioFrame> + 'af>,
    cross_fade: &'cf [CrossFadePair],
    cf_iter: Box<dyn Iterator<Item = &'cf CrossFadePair> + 'cf>,
    ad_segment: bool,
}

impl<'af, 'cf> Mixer for AdMixer<'af, 'cf> {
    fn content(&mut self, frame: &AudioFrame) -> AudioFrame {
        self.stop_ad_segment();

        let cf = self.cf_iter.next().unwrap();
        let ad = if cf.fade_out() > 0.0 {
            self.ad_iter
                .next()
                .cloned()
                .unwrap_or_else(|| codec::silence_frame(frame))
        } else {
            codec::silence_frame(frame)
        };

        (cf * (&ad, frame)).with_pts(frame.pts())
    }

    fn advertisement(&mut self, frame: &AudioFrame) -> AudioFrame {
        self.start_ad_segment();

        let cf = self.cf_iter.next().unwrap();
        let ad = if cf.fade_in() > 0.0 {
            self.ad_iter
                .next()
                .cloned()
                .unwrap_or_else(|| codec::silence_frame(frame))
        } else {
            codec::silence_frame(frame)
        };

        (cf * (frame, &ad)).with_pts(frame.pts())
    }
}

impl<'af, 'cf> AdMixer<'af, 'cf> {
    pub fn new(ad_frames: &'af [AudioFrame], cross_fade: &'cf [CrossFadePair]) -> Self {
        Self {
            ad_frames,
            ad_iter: Box::new(ad_frames.iter().cycle()),
            cross_fade,
            cf_iter: Box::new(repeat(&CrossFadePair::END)),
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
}

#[cfg(test)]
mod tests {
    use codec::dsp::{CrossFade, ParabolicCrossFade};

    use crate::mixer::tests::{new_frame_series, SamplesAsVec};
    use crate::mixer::{AdMixer, Mixer};

    #[test]
    fn test_music_to_advertisement() {
        let advertisement = new_frame_series(10, 0, 0.5);
        let music = new_frame_series(20, 10, 1.0);
        let cross_fade = ParabolicCrossFade::generate(4);

        let mut sut = AdMixer::new(&advertisement, &cross_fade);

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
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.6666667, 0.33333334, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5,
                0.5, 0.5, 0.33333334, 0.6666667, 1.0, 1.0
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
