use std::collections::VecDeque;

use codec::dsp::CrossFader;
use codec::{AudioFrame, Pts};

use crate::ad_provider::AdProvider;

use super::Mixer;

#[derive(Debug, Eq, PartialEq)]
enum Track {
    Main,
    Side,
}

pub struct AdsMixer {
    ads: Vec<AudioFrame>,
    _ad_provider: AdProvider,
    cross_fader: CrossFader,
    pts: Pts,
    main_track: VecDeque<AudioFrame>,
    side_track: VecDeque<AudioFrame>,
    side_buffer: VecDeque<AudioFrame>,
    active_track: Track,
}

impl Mixer for AdsMixer {
    fn push(&mut self, kind: analyzer::ContentKind, frame: &AudioFrame) -> AudioFrame {
        self.pts.update(frame);
        match kind {
            analyzer::ContentKind::Advertisement => self.advertisement(frame),
            analyzer::ContentKind::Music
            | analyzer::ContentKind::Talk
            | analyzer::ContentKind::Unknown => self.content(frame),
        }
    }
}

impl AdsMixer {
    pub fn new(ad_provider: AdProvider, cross_fader: CrossFader) -> Self {
        cross_fader.drain();
        Self {
            ads: (*ad_provider.next().unwrap()).clone(),
            _ad_provider: ad_provider,
            cross_fader,
            main_track: VecDeque::new(),
            side_track: VecDeque::new(),
            side_buffer: VecDeque::new(),
            pts: Pts::new(2_048, 48_000),
            active_track: Track::Main,
        }
    }

    fn pts(&mut self, frame: AudioFrame) -> AudioFrame {
        frame.with_pts(self.pts.next())
    }

    fn content(&mut self, frame: &AudioFrame) -> AudioFrame {
        self.main_track.push_back(frame.clone());
        self.side_buffer.clear();

        let output = if self.side_track.len() > self.cross_fader.len() {
            self.side_track.pop_front().unwrap()
        } else {
            if self.active_track == Track::Side {
                self.cross_fader.reset();
                self.active_track = Track::Main;
            }
            let ad = self
                .side_track
                .pop_front()
                .unwrap_or_else(|| codec::silence_frame(frame));
            let content = self.main_track.pop_front().unwrap();
            self.cross_fader.apply(&ad, &content)
        };
        self.pts(output)
    }

    fn advertisement(&mut self, frame: &AudioFrame) -> AudioFrame {
        self.side_buffer.push_back(frame.clone());

        let output = if self.main_track.is_empty() {
            if self.active_track == Track::Main {
                self.cross_fader.reset();
                self.active_track = Track::Side;
            }

            if self.side_track.is_empty() {
                self.side_track.extend(self.ads.clone());
            }

            let content = self
                .side_buffer
                .pop_front()
                .unwrap_or_else(|| codec::silence_frame(frame));
            let ad = self.side_track.pop_front().unwrap();
            self.cross_fader.apply(&content, &ad)
        } else {
            self.main_track.pop_front().unwrap()
        };
        self.pts(output)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use analyzer::ContentKind;
    use codec::dsp::{CrossFader, ParabolicCrossFade};
    use codec::{AudioFrame, Timestamp};

    use crate::ad_provider::AdProvider;
    use crate::routes::play::mixer::tests::{create_frames, pts_seq, SamplesAsVec};

    use super::{AdsMixer, Mixer};

    #[test]
    fn test_one_ads_block_short_buffer() {
        let mut player = Player::new(AdsMixer::new(
            AdProvider::new_testing(create_frames(10, 0.5)),
            CrossFader::exact::<ParabolicCrossFade>(4),
        ));
        player.content(5).advertisement(5).content(10).silence(2);

        assert_eq!(
            &player.samples(),
            &[
                // M
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                // CF
                1.0,
                0.666_666_7,
                0.333_333_34,
                0.5,
                // A
                0.5,
                0.5, // MT
                // CF
                0.5,
                0.333_333_34,
                0.666_666_7,
                1.0,
                // M
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                0.0
            ]
        );

        assert_eq!(player.timestamps(), pts_seq(22));
    }

    #[test]
    fn test_ads_blocks_overlaps() {
        let mut player = Player::new(AdsMixer::new(
            AdProvider::new_testing(create_frames(10, 0.5)),
            CrossFader::exact::<ParabolicCrossFade>(4),
        ));

        player
            .content(5)
            .advertisement(5)
            .content(5)
            .advertisement(5)
            .silence(7);

        assert_eq!(
            &player.samples(),
            &[
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                // CF
                1.0,
                0.666_666_7,
                0.333_333_34,
                0.5,
                // A
                0.5,
                0.5,
                // CF
                0.5,
                0.333_333_34,
                0.666_666_7,
                1.0,
                // M
                1.0,
                // CF
                1.0,
                0.666_666_7,
                0.333_333_34,
                0.5,
                // A
                0.5,
                0.5,
                // CF
                0.5,
                0.333_333_34,
                0.0,
                0.0,
                // S
                0.0
            ]
        );

        assert_eq!(player.timestamps(), pts_seq(27));
        assert!(player.mixer.side_track.is_empty());
        assert_eq!(2, player.mixer.main_track.len());
    }

    #[test]
    fn test_filled_buffer_skips_ads() {
        let mut player = Player::new(AdsMixer::new(
            AdProvider::new_testing(create_frames(10, 0.5)),
            CrossFader::exact::<ParabolicCrossFade>(2),
        ));

        player
            .content(1)
            .advertisement(1)
            .content(10)
            .advertisement(1)
            .silence(7);

        assert_eq!(
            &player.samples(),
            &[
                1.0, 1.0, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
                1.0, 1.0, 1.0, 0.0
            ]
        );

        assert_eq!(player.timestamps(), pts_seq(20));
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
