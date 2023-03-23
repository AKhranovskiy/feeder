pub type PlayerId = usize;

#[derive(Debug, Clone, Copy)]
pub enum PlayEvent {
    Start(PlayerId, time::OffsetDateTime),
    FirstQuarter(PlayerId, time::OffsetDateTime),
    Median(PlayerId, time::OffsetDateTime),
    ThirdQuarter(PlayerId, time::OffsetDateTime),
    End(PlayerId, time::OffsetDateTime),
}
