use codec::AudioFrame;

pub struct AdsProvider<'af> {
    ad_frames: &'af [AudioFrame],
    ad_iter: Box<dyn Iterator<Item = &'af AudioFrame> + 'af>,
    played: usize,
}

impl<'af> AdsProvider<'af> {
    pub fn new(frames: &'af [AudioFrame]) -> Self {
        Self {
            ad_frames: frames,
            ad_iter: Box::new(frames.iter()),
            played: 0,
        }
    }

    pub fn next(&mut self) -> Option<&'af AudioFrame> {
        if self.played == self.ad_frames.len() {
            self.restart();
        }
        self.played += 1;
        self.ad_iter.next()
    }

    pub fn remains(&self) -> usize {
        self.ad_frames.len() - self.played
    }

    pub fn restart(&mut self) {
        self.ad_iter = Box::new(self.ad_frames.iter());
        self.played = 0;
    }

    pub fn len(&self) -> usize {
        self.ad_frames.len()
    }
}
