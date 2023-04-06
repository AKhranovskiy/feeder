use std::collections::VecDeque;
use std::iter::repeat;

use codec::dsp::CrossFadePair;
use codec::{AudioFrame, Pts, Timestamp};

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
    last_played_timestamp: Timestamp,
}

impl<'af, 'cf> Mixer for AdsMixer<'af, 'cf> {
    fn push(&mut self, kind: analyzer::ContentKind, frame: AudioFrame) -> AudioFrame {
        match kind {
            analyzer::ContentKind::Advertisement => self.advertisement(frame),
            analyzer::ContentKind::Music
            | analyzer::ContentKind::Talk
            | analyzer::ContentKind::Unknown => self.content(frame),
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
            last_played_timestamp: ad_frames[0].pts(),
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

    fn check(&mut self, frame: AudioFrame) -> AudioFrame {
        if frame.pts() < self.last_played_timestamp {
            eprintln!(
                "OUT OF ORDER: played {:?}, next {:?}",
                self.last_played_timestamp,
                frame.pts()
            );
        }
        self.last_played_timestamp = frame.pts();
        frame
    }

    fn content(&mut self, frame: AudioFrame) -> AudioFrame {
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
        self.check(frame)
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
        self.check(frame)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use analyzer::ContentKind;
    use codec::dsp::{CrossFade, ParabolicCrossFade};
    use codec::{AudioFrame, Pts, Timestamp};

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
        pts: Pts,
    }

    impl<'m, 'af, 'cf> Player<'m, 'af, 'cf> {
        fn new(mixer: &'m mut AdsMixer<'af, 'cf>) -> Self {
            let frame = create_frames(1, 1.0)[0].clone();
            let pts = Pts::from(&frame);
            Self {
                mixer,
                frame,
                output: vec![],
                pts,
            }
        }

        fn content(&mut self, length: usize) -> &mut Self {
            self.output.extend((0..length).map(|_| {
                self.mixer.push(
                    ContentKind::Music,
                    self.frame.clone().with_pts(self.pts.next()),
                )
            }));
            self
        }

        fn advertisement(&mut self, length: usize) -> &mut Self {
            self.output.extend((0..length).map(|_| {
                self.mixer
                    .push(ContentKind::Advertisement, self.frame.clone())
            }));
            self
        }

        fn silence(&mut self, length: usize) -> &mut Self {
            self.output.extend(
                create_frames(length, 0.0)
                    .into_iter()
                    .map(|frame| self.mixer.push(ContentKind::Music, frame)),
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
