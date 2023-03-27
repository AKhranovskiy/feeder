use std::iter::repeat;

use codec::dsp::CrossFadePair;
use codec::{AudioFrame, Pts, Timestamp};

use super::Mixer;

pub struct SilenceMixer<'cf> {
    cross_fade: &'cf [CrossFadePair],
    cf_iter: Box<dyn Iterator<Item = &'cf CrossFadePair> + 'cf>,
    ad_segment: bool,
    pts: Pts,
}

impl<'cf> SilenceMixer<'cf> {
    pub fn new(cross_fade: &'cf [CrossFadePair]) -> Self {
        Self {
            cross_fade,
            cf_iter: Box::new(repeat(&CrossFadePair::END)),
            ad_segment: false,
            pts: Pts::new(2048, 48_000),
        }
    }

    fn pts(&mut self) -> Timestamp {
        self.pts.next()
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

impl<'cf> Mixer<'_> for SilenceMixer<'cf> {
    fn content(&mut self, frame: AudioFrame) -> AudioFrame {
        self.stop_ad_segment();
        let cf = self.cf_iter.next().unwrap();
        let silence = codec::silence_frame(&frame);
        (cf * (&silence, &frame)).with_pts(self.pts())
    }

    fn advertisement(&mut self, frame: AudioFrame) -> AudioFrame {
        self.start_ad_segment();
        let cf = self.cf_iter.next().unwrap();
        let silence = codec::silence_frame(&frame);
        (cf * (&frame, &silence)).with_pts(self.pts())
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use codec::dsp::{CrossFade, ParabolicCrossFade};

    use crate::routes::play::mixer::tests::{create_frames, pts_seq, SamplesAsVec};
    use crate::routes::play::mixer::{Mixer, SilenceMixer};

    #[test]
    fn test_music_to_advertisement() {
        let music = create_frames(20, 1.0);
        let cross_fade = ParabolicCrossFade::generate(3);

        let mut sut = SilenceMixer::new(&cross_fade);

        let mut output = vec![];

        output.extend(
            music
                .iter()
                .take(5)
                .cloned()
                .map(|frame| sut.content(frame)),
        );
        output.extend(
            music
                .iter()
                .skip(5)
                .take(10)
                .cloned()
                .map(|frame| sut.advertisement(frame)),
        );
        output.extend(
            music
                .iter()
                .skip(15)
                .cloned()
                .map(|frame| sut.content(frame)),
        );

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
            .map(codec::AudioFrame::pts)
            .collect::<Vec<_>>();

        assert_eq!(timestamps, pts_seq(20));
    }
}
