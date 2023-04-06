use std::collections::VecDeque;

use codec::dsp::CrossFader;
use codec::{AudioFrame, Pts};

use super::Mixer;

#[derive(Debug, Eq, PartialEq)]
enum Track {
    Main,
    Side,
}

pub struct AdsMixer {
    ads: Vec<AudioFrame>,
    cross_fader: CrossFader,
    pts: Pts,
    main_track: VecDeque<AudioFrame>,
    side_track: VecDeque<AudioFrame>,
    active_track: Track,
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
            main_track: VecDeque::new(),
            side_track: VecDeque::new(),
            pts: Pts::new(2_048, 48_000),
            active_track: Track::Main,
        }
    }

    fn pts(&mut self, frame: AudioFrame) -> AudioFrame {
        frame.with_pts(self.pts.next())
    }

    fn content(&mut self, frame: &AudioFrame) -> AudioFrame {
        self.main_track.push_back(frame.clone());

        let output = if self.side_track.len() > self.cross_fader.len() / 2 {
            self.side_track.pop_front().unwrap()
        } else if self.side_track.len() == self.cross_fader.len() / 2 && !self.side_track.is_empty()
        {
            self.active_track = Track::Main;

            self.cross_fader.reset();

            let content = self.main_track.pop_front().unwrap();
            let ad = self.side_track.pop_front().unwrap();

            self.cross_fader.apply(&ad, &content)
        } else if !self.side_track.is_empty() {
            let content = self.main_track.pop_front().unwrap();
            let ad = self.side_track.pop_front().unwrap();

            self.cross_fader.apply(&ad, &content)
        } else {
            let content = self.main_track.pop_front().unwrap();
            // side track is empty
            self.cross_fader.apply(&content, &content)
        };

        self.pts(output)
    }

    fn advertisement(&mut self, frame: &AudioFrame) -> AudioFrame {
        let output =
            if self.main_track.len() > self.cross_fader.len() / 2 && !self.main_track.is_empty() {
                self.main_track.pop_back().unwrap()
            } else if self.side_track.is_empty() {
                if Track::Main == self.active_track {
                    self.cross_fader.reset();
                    self.active_track = Track::Side;
                }

                self.side_track.extend(self.ads.clone());

                self.main_track.push_back(frame.clone());

                let content = self.main_track.pop_back().unwrap();
                let ad = self.side_track.pop_front().unwrap();

                self.cross_fader.apply(&content, &ad)
            } else {
                self.main_track.push_back(frame.clone());

                let content = self.main_track.pop_back().unwrap();
                let ad = self.side_track.pop_front().unwrap();

                self.cross_fader.apply(&content, &ad)
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

    use crate::routes::play::mixer::tests::{create_frames, pts_seq, SamplesAsVec};

    use super::{AdsMixer, Mixer};

    #[test]
    fn test_one_ads_block_short_buffer() {
        let mut player = Player::new(AdsMixer::new(
            create_frames(10, 0.5),
            CrossFader::exact::<ParabolicCrossFade>(4),
        ));
        player.content(5).advertisement(5).content(10).silence(4);

        assert_eq!(
            &player.samples(),
            &[
                1.0,
                0.666_666_7,
                0.666_666_7,
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

        assert_eq!(player.timestamps(), pts_seq(24));
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
