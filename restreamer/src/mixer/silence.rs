use std::iter::repeat;

use codec::dsp::CrossFadePair;
use codec::{AudioFrame, Pts, Timestamp};

use super::Mixer;

pub struct SilenceMixer<'cf> {
    cross_fade: &'cf [CrossFadePair],
    cf_iter: Box<dyn Iterator<Item = &'cf CrossFadePair> + 'cf>,
    ad_segment: bool,
    pts: Option<Pts>,
}

impl<'cf> SilenceMixer<'cf> {
    pub fn new(cross_fade: &'cf [CrossFadePair]) -> Self {
        Self {
            cross_fade,
            cf_iter: Box::new(repeat(&CrossFadePair::END)),
            ad_segment: false,
            pts: None,
        }
    }

    fn pts(&mut self, frame: &AudioFrame) -> Timestamp {
        if self.pts.is_none() {
            self.pts = Some(Pts::from(frame));
        }

        self.pts.as_mut().unwrap().next()
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
        (cf * (&silence, frame)).with_pts(self.pts(frame))
    }

    fn advertisement(&mut self, frame: &AudioFrame) -> AudioFrame {
        self.start_ad_segment();
        let cf = self.cf_iter.next().unwrap();
        let silence = codec::silence_frame(frame);
        (cf * (frame, &silence)).with_pts(self.pts(frame))
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use codec::dsp::{CrossFade, ParabolicCrossFade};

    use crate::mixer::tests::{create_frames, pts_seq, SamplesAsVec};
    use crate::mixer::{Mixer, SilenceMixer};

    #[test]
    fn test_music_to_advertisement() {
        let music = create_frames(20, 1.0);
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

        let timestamps = output.iter().map(|frame| frame.pts()).collect::<Vec<_>>();

        assert_eq!(timestamps, pts_seq(20));
    }
}
