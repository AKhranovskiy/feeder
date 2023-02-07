use std::iter::repeat;

use analyzer::ContentKind;
use codec::dsp::CrossFadePair;
use codec::AudioFrame;

use crate::play_params::PlayAction;

pub struct Mixer<'a> {
    action: PlayAction,
    ad_frames: &'a [AudioFrame],
    ad_iter: Box<dyn Iterator<Item = &'a AudioFrame> + 'a>,
    cross_fade: &'a [CrossFadePair],
    cf_iter: Box<dyn Iterator<Item = &'a CrossFadePair> + 'a>,
    ad_segment: bool,
}

impl<'a> Mixer<'a> {
    pub fn new(
        action: PlayAction,
        ad_frames: &'a [AudioFrame],
        cross_fade: &'a [CrossFadePair],
    ) -> Self {
        Self {
            action,
            ad_frames,
            ad_iter: Box::new(ad_frames.iter().cycle()),
            cross_fade,
            cf_iter: Box::new(cross_fade.iter().chain(repeat(&CrossFadePair::END))),
            ad_segment: false,
        }
    }

    fn start_ad_segment(&mut self) {
        if !self.ad_segment {
            self.ad_iter = Box::new(self.ad_frames.iter().cycle());
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

    pub fn push(&mut self, frame: AudioFrame, kind: ContentKind) -> AudioFrame {
        let pts = frame.pts();

        match kind {
            ContentKind::Music | ContentKind::Talk | ContentKind::Unknown => {
                self.stop_ad_segment();

                match self.action {
                    PlayAction::Passthrough => frame,
                    PlayAction::Silence => {
                        let silence = codec::silence_frame(&frame);
                        self.cf_iter.next().unwrap() * (&silence, &frame)
                    }
                    PlayAction::Lang(_) => {
                        let cf = self.cf_iter.next().unwrap();
                        let ad = if cf.fade_out() > 0.0 {
                            self.ad_iter
                                .next()
                                .cloned()
                                .unwrap_or_else(|| codec::silence_frame(&frame))
                        } else {
                            codec::silence_frame(&frame)
                        };
                        cf * (&ad, &frame)
                    }
                }
            }
            ContentKind::Advertisement => {
                self.start_ad_segment();

                match self.action {
                    PlayAction::Passthrough => frame,
                    PlayAction::Silence => {
                        let cf = self.cf_iter.next().unwrap();
                        let silence = codec::silence_frame(&frame);
                        cf * (&silence, &frame)
                    }
                    PlayAction::Lang(_) => {
                        let cf = self.cf_iter.next().unwrap();
                        let ad = if cf.fade_in() > 0.0 {
                            self.ad_iter
                                .next()
                                .cloned()
                                .unwrap_or_else(|| codec::silence_frame(&frame))
                        } else {
                            codec::silence_frame(&frame)
                        };
                        cf * (&frame, &ad)
                    }
                }
            }
        }
        .with_pts(pts)
    }
}
