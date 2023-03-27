use std::collections::VecDeque;
use std::iter::repeat;

use codec::dsp::CrossFadePair;
use codec::{AudioFrame, Pts};

use crate::adbuffet::AdBuffet;

use super::ads_provider::AdsProvider;
use super::Mixer;

pub struct AdsMixer<'ad, 'cf: 'ad> {
    ads: Box<AdsProvider<'ad>>,
    cross_fade: &'cf [CrossFadePair],
    cf_iter: Box<dyn Iterator<Item = &'cf CrossFadePair> + 'cf>,
    ad_segment: bool,
    play_buffer: VecDeque<AudioFrame>,
    pts: Pts,
    drain_play_buffer: bool,
    // last_played_timestamp: Timestamp,
}

impl<'ad, 'cf: 'ad> AdsMixer<'ad, 'cf> {
    pub fn new(buffet: &'ad AdBuffet, cross_fade: &'cf [CrossFadePair]) -> Self {
        Self {
            ads: Box::new(AdsProvider::new(buffet)),
            cross_fade,
            cf_iter: Box::new(repeat(&CrossFadePair::END)),
            ad_segment: false,
            play_buffer: VecDeque::new(),
            pts: Pts::new(2_048, 48_000),
            drain_play_buffer: false,
            // last_played_timestamp: ad_frames[0].pts(),
        }
    }

    fn start_ad_segment(&mut self) {
        if !self.ad_segment {
            // self.ads.start();
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

impl<'ad, 'cf: 'ad, 's: 'cf> Mixer<'s> for AdsMixer<'ad, 'cf> {
    fn content(&'s mut self, frame: AudioFrame) -> AudioFrame {
        self.play_buffer.push_back(frame.clone());

        // eprintln!(
        //     "BUFFER content {:?}",
        //     self.play_buffer
        //         .iter()
        //         .map(codec::AudioFrame::pts)
        //         .collect::<Vec<_>>()
        // );

        let frame = if self.ad_segment && self.ads.remains() > self.cross_fade.len() / 2 {
            self.advertisement(frame)
        } else {
            self.stop_ad_segment();

            let cf = self.cf_iter.next().unwrap();
            let ad = if cf.fade_out() > 0.0 {
                self.ads
                    .next()
                    .cloned()
                    .unwrap_or_else(|| codec::silence_frame(&frame))
            } else {
                codec::silence_frame(&frame)
            };
            let frame = self.play_buffer.pop_front().unwrap();
            // eprintln!("CONTENT {:?}", frame.pts());
            (cf * (&ad, &frame)).with_pts(self.pts.next())
        };

        frame
    }

    fn advertisement(&mut self, frame: AudioFrame) -> AudioFrame {
        // eprintln!(
        //     "BUFFER ads {:?}",
        //     self.play_buffer
        //         .iter()
        //         .map(codec::AudioFrame::pts)
        //         .collect::<Vec<_>>()
        // );

        if self.play_buffer.is_empty() {
            self.drain_play_buffer = false;
        }

        if !self.drain_play_buffer && !self.ad_segment {
            self.drain_play_buffer = self.play_buffer.len() > self.ads.len();
        }

        let frame = if self.drain_play_buffer {
            let frame = self.play_buffer.pop_front().unwrap();
            // eprintln!("DRAIN {:?}", frame.pts());
            frame.with_pts(self.pts.next())
        } else {
            self.start_ad_segment();

            let cf = self.cf_iter.next().unwrap();
            let ad = if cf.fade_in() > 0.0 {
                self.ads
                    .next()
                    .cloned()
                    .unwrap_or_else(|| codec::silence_frame(&frame))
            } else {
                codec::silence_frame(&frame)
            };

            (cf * (&frame, &ad)).with_pts(self.pts.next())
        };
        frame
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use codec::dsp::{CrossFade, ParabolicCrossFade};
    use codec::{AudioFrame, Pts, Timestamp};

    use crate::adbuffet::AdBuffet;
    use crate::routes::play::mixer::tests::{create_frames, pts_seq, SamplesAsVec};
    use crate::routes::play::mixer::{AdsMixer, Mixer};

    #[test]
    fn test_one_ads_block_short_buffer() {
        // let advertisement = create_frames(10, 0.5);
        let buffet = AdBuffet::empty();
        let cross_fade = ParabolicCrossFade::generate(4);
        let mut player = Player::new(AdsMixer::new(&buffet, &cross_fade));
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
        let buffet = AdBuffet::empty();
        let cross_fade = ParabolicCrossFade::generate(4);
        let mut player = Player::new(AdsMixer::new(&buffet, &cross_fade));

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
        let buffet = AdBuffet::empty();
        let cross_fade = ParabolicCrossFade::generate(2);
        let mut player = Player::new(AdsMixer::new(&buffet, &cross_fade));

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

    struct Player<'af, 'cf> {
        mixer: AdsMixer<'af, 'cf>,
        frame: AudioFrame,
        output: Vec<AudioFrame>,
        pts: Pts,
    }

    impl<'af, 'cf: 'af> Player<'af, 'cf> {
        fn new(mixer: AdsMixer<'af, 'cf>) -> Self {
            let frame = create_frames(1, 1.0)[0].clone();
            let pts = Pts::new(4, 4);
            Self {
                mixer,
                frame,
                output: vec![],
                pts,
            }
        }

        fn content(&mut self, length: usize) -> &mut Self {
            let frames = (0..length)
                .map(|_| self.frame.clone().with_pts(self.pts.next()))
                .collect::<Vec<_>>();

            let frames = frames
                .into_iter()
                .map(|f| self.mixer.content(f))
                .collect::<Vec<_>>();

            self.output.extend(frames.into_iter());
            self
        }

        fn advertisement(&mut self, length: usize) -> &mut Self {
            self.output
                .extend((0..length).map(|_| self.mixer.advertisement(self.frame.clone())));
            self
        }

        fn silence(&mut self, length: usize) -> &mut Self {
            self.output.extend(
                create_frames(length, 0.0)
                    .into_iter()
                    .map(|frame| self.mixer.content(frame)),
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
