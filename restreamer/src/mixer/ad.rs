use std::collections::VecDeque;
use std::iter::repeat;
use std::time::Duration;

use codec::dsp::CrossFadePair;
use codec::{AudioFrame, FrameDuration, Timestamp};

use super::Mixer;

pub struct AdMixer<'af, 'cf> {
    ads: Box<AdsBox<'af>>,
    cross_fade: &'cf [CrossFadePair],
    cf_iter: Box<dyn Iterator<Item = &'cf CrossFadePair> + 'cf>,
    ad_segment: bool,
    play_buffer: VecDeque<AudioFrame>,
    pts: Timestamp,
    duration: Duration,
}

struct AdsBox<'af> {
    ad_frames: &'af [AudioFrame],
    ad_iter: Box<dyn Iterator<Item = &'af AudioFrame> + 'af>,
    played: usize,
}

impl<'af> AdsBox<'af> {
    fn new(frames: &'af [AudioFrame]) -> Self {
        Self {
            ad_frames: frames,
            ad_iter: Box::new(frames.iter()),
            played: 0,
        }
    }

    fn next(&mut self) -> Option<&'af AudioFrame> {
        if self.played == self.ad_frames.len() {
            self.reset()
        }
        self.played += 1;
        self.ad_iter.next()
    }

    fn left(&self) -> usize {
        self.ad_frames.len() - self.played
    }

    fn reset(&mut self) {
        self.ad_iter = Box::new(self.ad_frames.iter());
        self.played = 0;
    }

    fn len(&self) -> usize {
        self.ad_frames.len()
    }
}

impl<'af, 'cf> Mixer for AdMixer<'af, 'cf> {
    fn content(&mut self, frame: &AudioFrame) -> AudioFrame {
        self.play_buffer.push_back(frame.clone());

        if self.ad_segment && self.ads.left() > self.cross_fade.len() / 2 {
            self.advertisement(frame)
        } else {
            self.stop_ad_segment();

            let cf = self.cf_iter.next().unwrap();
            let ad = if cf.fade_out() > 0.0 {
                self.ads
                    .next()
                    .cloned()
                    .unwrap_or_else(|| codec::silence_frame(frame))
            } else {
                codec::silence_frame(frame)
            };

            (cf * (&ad, self.play_buffer.pop_front().as_ref().unwrap_or(frame)))
                .with_pts(self.pts())
        }
    }

    fn advertisement(&mut self, frame: &AudioFrame) -> AudioFrame {
        if !self.ad_segment && self.play_buffer.len() > self.ads.len() {
            self.content(frame)
        } else {
            self.start_ad_segment();

            let cf = self.cf_iter.next().unwrap();
            let ad = if cf.fade_in() > 0.0 {
                self.ads
                    .next()
                    .cloned()
                    .unwrap_or_else(|| codec::silence_frame(frame))
            } else {
                codec::silence_frame(frame)
            };

            (cf * (frame, &ad)).with_pts(self.pts())
        }
    }
}

impl<'af, 'cf> AdMixer<'af, 'cf> {
    pub fn new(ad_frames: &'af [AudioFrame], cross_fade: &'cf [CrossFadePair]) -> Self {
        Self {
            ads: Box::new(AdsBox::new(ad_frames)),
            cross_fade,
            cf_iter: Box::new(repeat(&CrossFadePair::END)),
            ad_segment: false,
            play_buffer: VecDeque::new(),
            pts: Timestamp::new(0, ad_frames[0].time_base()),
            duration: ad_frames[0].duration(),
        }
    }

    fn pts(&mut self) -> Timestamp {
        let pts = self.pts;
        self.pts += self.duration;
        pts
    }

    fn start_ad_segment(&mut self) {
        if !self.ad_segment {
            self.ads.reset();
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
#[allow(clippy::float_cmp)]
mod tests {
    use codec::dsp::{CrossFade, ParabolicCrossFade};

    use crate::mixer::tests::{new_frame_series, SamplesAsVec};
    use crate::mixer::{AdMixer, Mixer};

    #[test]
    fn test_one_ads_block_short_buffer() {
        let advertisement = new_frame_series(10, 0, 0.5);
        let music = new_frame_series(20, 10, 1.0);
        let silence = new_frame_series(6, 30, 0.0);

        let cross_fade = ParabolicCrossFade::generate(4);

        let mut sut = AdMixer::new(&advertisement, &cross_fade);

        let mut output = vec![];

        output.extend(music.iter().take(5).map(|frame| sut.content(frame)));
        output.extend(
            music
                .iter()
                .skip(5)
                .take(5)
                .map(|frame| sut.advertisement(frame)),
        );
        output.extend(music.iter().skip(10).map(|frame| sut.content(frame)));
        output.extend(silence.iter().map(|frame| sut.content(frame)));

        let samples = output
            .iter()
            .flat_map(|frame| frame.samples_as_vec().into_iter())
            .collect::<Vec<_>>();

        assert_eq!(
            &samples,
            &[
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.6666667, 0.33333334, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5,
                0.5, 0.5, 0.33333334, 0.6666667, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0
            ]
        );

        let timestamps = output
            .iter()
            .map(|frame| frame.pts().as_secs().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(timestamps, (0..=25).collect::<Vec<_>>());
    }

    #[test]
    fn test_ads_blocks_overlaps() {
        let advertisement = new_frame_series(10, 0, 0.5);
        let cross_fade = ParabolicCrossFade::generate(4);
        let mut sut = AdMixer::new(&advertisement, &cross_fade);

        let music_block_a = new_frame_series(5, 0, 1.0).into_iter();
        let music_block_b = new_frame_series(5, 5, 1.0).into_iter();
        let music_block_c = new_frame_series(5, 10, 1.0).into_iter();
        let music_block_d = new_frame_series(5, 10, 1.0).into_iter();

        let silence = new_frame_series(11, 15, 0.0).into_iter();

        let mut output = vec![];
        output.extend(music_block_a.map(|frame| sut.content(&frame)));
        output.extend(music_block_b.map(|frame| sut.advertisement(&frame)));
        output.extend(music_block_c.map(|frame| sut.content(&frame)));
        output.extend(music_block_d.map(|frame| sut.advertisement(&frame)));

        output.extend(silence.map(|frame| sut.content(&frame)));

        let samples = output
            .iter()
            .flat_map(|frame| frame.samples_as_vec().into_iter())
            .collect::<Vec<_>>();

        assert_eq!(
            &samples,
            &[
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.6666667, 0.33333334, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5,
                0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.33333334, 0.6666667,
                1.0, 1.0, 0.0
            ]
        );

        let timestamps = output
            .iter()
            .map(|frame| frame.pts().as_secs().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(timestamps, (0..=30).collect::<Vec<_>>());
    }
}
