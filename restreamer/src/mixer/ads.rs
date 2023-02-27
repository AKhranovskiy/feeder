use std::collections::VecDeque;
use std::iter::repeat;

use codec::dsp::CrossFadePair;
use codec::{AudioFrame, Pts};

use super::ads_provider::AdsProvider;
use super::Mixer;

pub struct AdsMixer<'af, 'cf> {
    ads: Box<AdsProvider<'af>>,
    cross_fade: &'cf [CrossFadePair],
    cf_iter: Box<dyn Iterator<Item = &'cf CrossFadePair> + 'cf>,
    ad_segment: bool,
    play_buffer: VecDeque<AudioFrame>,
    pts: Pts,
    drain_play_buffer: bool,
}

impl<'af, 'cf> Mixer for AdsMixer<'af, 'cf> {
    fn content(&mut self, frame: &AudioFrame) -> AudioFrame {
        self.play_buffer.push_back(frame.clone());

        if self.ad_segment && self.ads.remains() > self.cross_fade.len() / 2 {
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
                .with_pts(self.pts.next())
        }
    }

    fn advertisement(&mut self, frame: &AudioFrame) -> AudioFrame {
        if self.play_buffer.is_empty() {
            self.drain_play_buffer = false;
        }

        if !self.drain_play_buffer && !self.ad_segment {
            self.drain_play_buffer = self.play_buffer.len() > self.ads.len();
        }

        if self.drain_play_buffer {
            let last_frame = self.play_buffer.pop_back().unwrap();
            self.content(&last_frame)
        } else {
            // TODO New add should not start if there is big enough play buffer.
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

            (cf * (frame, &ad)).with_pts(self.pts.next())
        }
    }
}

impl<'af, 'cf> AdsMixer<'af, 'cf> {
    pub fn new(ad_frames: &'af [AudioFrame], cross_fade: &'cf [CrossFadePair]) -> Self {
        Self {
            ads: Box::new(AdsProvider::new(ad_frames)),
            cross_fade,
            cf_iter: Box::new(repeat(&CrossFadePair::END)),
            ad_segment: false,
            play_buffer: VecDeque::new(),
            pts: Pts::from(&ad_frames[0]),
            drain_play_buffer: false,
        }
    }

    fn start_ad_segment(&mut self) {
        if !self.ad_segment {
            self.ads.restart();
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
    use codec::{AudioFrame, Timestamp};

    use crate::mixer::tests::{create_frames, pts_seq, SamplesAsVec};
    use crate::mixer::{AdsMixer, Mixer};

    #[test]
    fn test_one_ads_block_short_buffer() {
        let advertisement = create_frames(10, 0.5);
        let cross_fade = ParabolicCrossFade::generate(4);
        let mut sut = AdsMixer::new(&advertisement, &cross_fade);

        let mut player = Player::new(&mut sut);
        player.content(5).advertisement(5).content(10).silence(6);

        assert_eq!(
            &player.samples(),
            &[
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                0.666_666_7,
                0.333_333_34,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.333_333_34,
                0.666_666_7,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                0.0
            ]
        );

        assert_eq!(player.timestamps(), pts_seq(26));
    }

    #[test]
    fn test_ads_blocks_overlaps() {
        let advertisement = create_frames(10, 0.5);
        let cross_fade = ParabolicCrossFade::generate(4);
        let mut sut = AdsMixer::new(&advertisement, &cross_fade);
        let mut player = Player::new(&mut sut);

        player
            .content(5)
            .advertisement(5)
            .content(5)
            .advertisement(5)
            .silence(11);

        assert_eq!(
            &player.samples(),
            &[
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                0.666_666_7,
                0.333_333_34,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.5,
                0.333_333_34,
                0.666_666_7,
                1.0,
                1.0,
                0.0
            ]
        );

        assert_eq!(player.timestamps(), pts_seq(31));
    }

    #[test]
    fn test_filled_buffer_skips_ads() {
        let advertisement = create_frames(5, 0.5);
        let cross_fade = ParabolicCrossFade::generate(2);

        let mut sut = AdsMixer::new(&advertisement, &cross_fade);
        let mut player = Player::new(&mut sut);

        player
            .content(1)
            .advertisement(3) // buffer 2
            .content(3)
            .advertisement(3) // buffer size 4
            .content(3)
            .advertisement(3) // buffer size 6
            .content(3)
            .advertisement(3) // plays buffer
            .silence(4);

        assert_eq!(
            &player.samples(),
            &[
                1.0, 1.0, 0.5, 0.5, 0.5, 0.5, 0.5, 1.0, 0.5, 0.5, 0.5, 0.5, 0.5, 1.0, 0.5, 0.5,
                0.5, 0.5, 0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0
            ]
        );

        assert_eq!(player.timestamps(), pts_seq(26));
    }

    struct Player<'m, 'af, 'cf> {
        mixer: &'m mut AdsMixer<'af, 'cf>,
        frame: AudioFrame,
        output: Vec<AudioFrame>,
    }

    impl<'m, 'af, 'cf> Player<'m, 'af, 'cf> {
        fn new(mixer: &'m mut AdsMixer<'af, 'cf>) -> Self {
            Self {
                mixer,
                frame: create_frames(1, 1.0)[0].clone(),
                output: vec![],
            }
        }

        fn content(&mut self, length: usize) -> &mut Self {
            self.output
                .extend((0..length).map(|_| self.mixer.content(&self.frame)));
            self
        }

        fn advertisement(&mut self, length: usize) -> &mut Self {
            self.output
                .extend((0..length).map(|_| self.mixer.advertisement(&self.frame)));
            self
        }

        fn silence(&mut self, length: usize) -> &mut Self {
            self.output.extend(
                create_frames(length, 0.0)
                    .into_iter()
                    .map(|frame| self.mixer.content(&frame)),
            );
            self
        }

        fn samples(&self) -> Vec<f32> {
            self.output
                .iter()
                .flat_map(|frame| frame.samples_as_vec().into_iter())
                .collect()
        }

        fn timestamps(&self) -> Vec<Timestamp> {
            self.output.iter().map(codec::AudioFrame::pts).collect()
        }
    }
}
