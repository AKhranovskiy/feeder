use std::collections::VecDeque;

use codec::dsp::CrossFader;
use codec::{AudioFrame, Pts};

use super::Mixer;

pub struct AdsMixer {
    ads: Vec<AudioFrame>,
    cross_fader: CrossFader,
    ad_segment: bool,
    play_buffer: VecDeque<AudioFrame>,
    pts: Pts,
}

impl Mixer for AdsMixer {
    fn push(&mut self, kind: analyzer::ContentKind, frame: &AudioFrame) -> AudioFrame {
        match kind {
            analyzer::ContentKind::Advertisement => self.advertisement(frame),
            analyzer::ContentKind::Music
            | analyzer::ContentKind::Talk
            | analyzer::ContentKind::Unknown => self.content(frame),
        }
    }
}

impl AdsMixer {
    pub fn new(ad_frames: Vec<AudioFrame>, cross_fader: CrossFader) -> Self {
        Self {
            ads: ad_frames,
            cross_fader,
            ad_segment: false,
            play_buffer: VecDeque::new(),
            pts: Pts::new(2_048, 48_000),
        }
    }

    fn start_ad_segment(&mut self) {
        if !self.ad_segment {
            self.cross_fader.reset();
            self.ad_segment = true;
            self.play_buffer.extend(self.ads.clone());
        }
    }

    fn stop_ad_segment(&mut self) {
        if self.ad_segment {
            self.cross_fader.reset();
            self.ad_segment = false;
        }
    }

    fn content(&mut self, frame: &AudioFrame) -> AudioFrame {
        self.stop_ad_segment();

        self.play_buffer.push_back(frame.clone());

        // let frame = if self.ad_segment && self.ads.remains() > self.cross_fade.len() / 2 {
        //     self.advertisement(frame)
        // } else {
        //     self.stop_ad_segment();
        //
        //     let cf = self.cf_iter.next().unwrap();
        //     let ad = if cf.fade_out() > 0.0 {
        //         self.ads
        //             .next()
        //             .cloned()
        //             .unwrap_or_else(|| codec::silence_frame(frame))
        //     } else {
        //         codec::silence_frame(frame)
        //     };
        //     let frame = self.play_buffer.pop_front().unwrap();
        //     (cf * (&ad, &frame)).with_pts(self.pts.next())
        // };
        // self.check(frame)
        self.play_buffer
            .pop_front()
            .unwrap()
            .with_pts(self.pts.next())
    }

    fn advertisement(&mut self, _frame: &AudioFrame) -> AudioFrame {
        self.start_ad_segment();

        // eprintln!(
        //     "BUFFER ads {:?}",
        //     self.play_buffer
        //         .iter()
        //         .map(codec::AudioFrame::pts)
        //         .collect::<Vec<_>>()
        // );

        // if self.play_buffer.is_empty() {
        //     self.drain_play_buffer = false;
        // }
        //
        // if !self.drain_play_buffer && !self.ad_segment {
        //     self.drain_play_buffer = self.play_buffer.len() > self.ads.len();
        // }
        //
        // let frame = if self.drain_play_buffer {
        //     let frame = self.play_buffer.pop_front().unwrap();
        //     // eprintln!("DRAIN {:?}", frame.pts());
        //     frame.with_pts(self.pts.next())
        // } else {
        //     self.start_ad_segment();
        //
        //     let cf = self.cf_iter.next().unwrap();
        //     let ad = if cf.fade_in() > 0.0 {
        //         self.ads
        //             .next()
        //             .cloned()
        //             .unwrap_or_else(|| codec::silence_frame(frame))
        //     } else {
        //         codec::silence_frame(frame)
        //     };
        //
        //     (cf * (frame, &ad)).with_pts(self.pts.next())
        // };
        // self.check(frame)
        if self.play_buffer.is_empty() {
            self.play_buffer.extend(self.ads.clone());
        }

        self.play_buffer.pop_back().unwrap()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use analyzer::ContentKind;
    use codec::dsp::{CrossFader, ParabolicCrossFade};
    use codec::{AudioFrame, Timestamp};

    use crate::routes::play::mixer::tests::{create_frames, pts_seq, SamplesAsVec};

    use super::{AdsMixer, Mixer};

    #[test]
    fn test_one_ads_block_short_buffer() {
        let advertisement = create_frames(10, 0.5);
        let mut player = Player::new(AdsMixer::new(
            advertisement,
            CrossFader::exact::<ParabolicCrossFade>(4),
        ));
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
        let mut player = Player::new(AdsMixer::new(
            create_frames(10, 0.5),
            CrossFader::exact::<ParabolicCrossFade>(4),
        ));

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
        let mut player = Player::new(AdsMixer::new(
            create_frames(5, 0.5),
            CrossFader::exact::<ParabolicCrossFade>(2),
        ));

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

    struct Player {
        mixer: AdsMixer,
        frame: AudioFrame,
        output: Vec<AudioFrame>,
    }

    impl Player {
        fn new(mixer: AdsMixer) -> Self {
            let frame = create_frames(1, 1.0)[0].clone();
            Self {
                mixer,
                frame,
                output: vec![],
            }
        }

        fn content(&mut self, length: usize) -> &mut Self {
            self.output
                .extend((0..length).map(|_| self.mixer.push(ContentKind::Music, &self.frame)));
            self
        }

        fn advertisement(&mut self, length: usize) -> &mut Self {
            self.output.extend(
                (0..length).map(|_| self.mixer.push(ContentKind::Advertisement, &self.frame)),
            );
            self
        }

        fn silence(&mut self, length: usize) -> &mut Self {
            self.output.extend(
                create_frames(length, 0.0)
                    .into_iter()
                    .map(|frame| self.mixer.push(ContentKind::Music, &frame)),
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
