use std::collections::VecDeque;

use axum::async_trait;
use codec::dsp::CrossFader;
use codec::{AudioFrame, Pts};

use crate::ads_management::AdsPlanner;

use super::Mixer;

#[derive(Debug, Eq, PartialEq)]
enum Track {
    Main,
    Side,
}

pub struct AdsMixer {
    ads_planner: AdsPlanner,
    cross_fader: CrossFader,
    pts: Pts,
    main_track: VecDeque<AudioFrame>,
    side_track: VecDeque<AudioFrame>,
    side_buffer: VecDeque<AudioFrame>,
    active_track: Track,
}

#[async_trait]
impl Mixer for AdsMixer {
    async fn push(&mut self, kind: analyzer::ContentKind, frame: &AudioFrame) -> AudioFrame {
        self.pts.update(frame);
        match kind {
            analyzer::ContentKind::Advertisement => self.advertisement(frame).await,
            analyzer::ContentKind::Music
            | analyzer::ContentKind::Talk
            | analyzer::ContentKind::Unknown => self.content(frame).await,
        }
    }
}

impl AdsMixer {
    pub fn new(ads_planner: AdsPlanner, pts: Pts, cross_fader: CrossFader) -> Self {
        cross_fader.drain();
        Self {
            ads_planner,
            cross_fader,
            main_track: VecDeque::new(),
            side_track: VecDeque::new(),
            side_buffer: VecDeque::new(),
            pts,
            active_track: Track::Main,
        }
    }

    fn pts(&mut self, frame: AudioFrame) -> AudioFrame {
        frame.with_pts(self.pts.next())
    }

    async fn content(&mut self, frame: &AudioFrame) -> AudioFrame {
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

            if self.side_track.is_empty() {
                self.ads_planner.finished().await;
            }
            self.cross_fader.apply(&ad, &content)
        };
        self.pts(output)
    }

    async fn advertisement(&mut self, frame: &AudioFrame) -> AudioFrame {
        self.side_buffer.push_back(frame.clone());

        let output = if self.main_track.is_empty() {
            if self.active_track == Track::Main {
                self.cross_fader.reset();
                self.active_track = Track::Side;
            }

            if self.side_track.is_empty() {
                self.ads_planner.finished().await;
                self.side_track
                    .extend(self.ads_planner.next().await.unwrap());
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
    use std::time::Duration;

    use analyzer::ContentKind;
    use codec::dsp::{CrossFader, ParabolicCrossFade};
    use codec::{AudioFrame, Pts, Timestamp};
    use nearly::assert_nearly_eq;

    use crate::ads_management::AdsPlanner;
    use crate::routes::play::mixer::tests::{create_frames, pts_seq, SamplesAsVec};

    use super::{AdsMixer, Mixer};

    const PTS: Pts = Pts::const_new(Duration::from_secs(1));

    #[tokio::test]
    async fn test_one_ads_block_short_buffer() {
        let mut player = Player::new(AdsMixer::new(
            AdsPlanner::testing(create_frames(10, 0.5)).await,
            PTS,
            CrossFader::exact::<ParabolicCrossFade>(4),
        ));
        player
            .content(5)
            .await
            .advertisement(5)
            .await
            .content(10)
            .await
            .silence(2)
            .await;

        #[rustfmt::skip]
        assert_nearly_eq!(
            player.samples(),
            [
                // M
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                // CF
                1.0,
                0.666,
                0.333,
                0.5,
                // A
                0.5,
                0.5, // MT
                // CF
                0.5,
                0.333,
                0.666,
                1.0,
                // M
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                0.0
            ],
            eps = 1e-3
        );

        assert_eq!(player.timestamps(), pts_seq(22));
    }

    #[tokio::test]
    async fn test_ads_blocks_overlaps() {
        let mut player = Player::new(AdsMixer::new(
            AdsPlanner::testing(create_frames(10, 0.5)).await,
            PTS,
            CrossFader::exact::<ParabolicCrossFade>(4),
        ));

        player
            .content(5)
            .await
            .advertisement(5)
            .await
            .content(5)
            .await
            .advertisement(5)
            .await
            .silence(7)
            .await;

        #[rustfmt::skip]
        assert_nearly_eq!(
            player.samples(),
            [
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                // CF
                1.0,
                0.666,
                0.333,
                0.5,
                // A
                0.5,
                0.5,
                // CF
                0.5,
                0.333,
                0.666,
                1.0,
                // M
                1.0,
                // CF
                1.0,
                0.666,
                0.333,
                0.5,
                // A
                0.5,
                0.5,
                // CF
                0.5,
                0.333,
                0.0,
                0.0,
                // S
                0.0
            ],
            eps = 1e-3
        );

        assert_eq!(player.timestamps(), pts_seq(27));
        assert!(player.mixer.side_track.is_empty());
        assert_eq!(2, player.mixer.main_track.len());
    }

    #[tokio::test]
    async fn test_filled_buffer_skips_ads() {
        let mut player = Player::new(AdsMixer::new(
            AdsPlanner::testing(create_frames(10, 0.5)).await,
            PTS,
            CrossFader::exact::<ParabolicCrossFade>(2),
        ));

        player
            .content(1)
            .await
            .advertisement(1)
            .await
            .content(10)
            .await
            .advertisement(1)
            .await
            .silence(7)
            .await;

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

        async fn content(&mut self, length: usize) -> &mut Self {
            for _ in 0..length {
                self.output
                    .push(self.mixer.push(ContentKind::Music, &self.frame).await);
            }
            self
        }

        async fn advertisement(&mut self, length: usize) -> &mut Self {
            for _ in 0..length {
                self.output.push(
                    self.mixer
                        .push(ContentKind::Advertisement, &self.frame)
                        .await,
                );
            }
            self
        }

        async fn silence(&mut self, length: usize) -> &mut Self {
            for frame in create_frames(length, 0.0) {
                self.output
                    .push(self.mixer.push(ContentKind::Music, &frame).await);
            }
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
