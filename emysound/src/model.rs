use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    /// Unique ID for a query match. You can use this ID to search for query matches in Emy /api/v1/matches endpoint.
    pub id: String,
    /// Object containing track information.
    pub track: TrackInfo,
    /// Query match object.
    pub audio: Option<AudioMatch>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    /// Track unique identifier.
    pub id: String,
    /// Track title.
    pub title: Option<String>,
    /// Track artist.
    pub artist: Option<String>,
    /// Audio track length, measured in seconds.
    #[serde(rename = "audioTrackLength")]
    pub length: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioMatch {
    /// Query match unique identifier.
    #[serde(rename = "queryMatchId")]
    pub id: String,
    /// Object containing information about query match coverage.
    pub coverage: AudioCoverage,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioCoverage {
    /// Query match starting position in seconds.
    pub query_match_starts_at: f32,
    /// Track match starting position in seconds.
    pub track_match_starts_at: f32,
    /// Gets relative query coverage, calculated by dividing QueryCoverageLength by QueryLength.
    pub query_coverage: Option<f32>,
    /// Gets relative track coverage, calculated by dividing TrackCoverageLength by TrackLength.
    pub track_coverage: Option<f32>,
    /// Query coverage length in seconds. Shows how many seconds from the query have been covered in the track.
    pub query_coverage_length: f32,
    /// Track coverage length in seconds. Shows how many seconds form the track have been covered in the query.
    pub track_coverage_length: f32,
    /// Discrete query coverage length in seconds. It is calculated by summing QueryCoverageLength with QueryGaps.
    pub query_discrete_coverage_length: f32,
    /// Discrete track coverage length in seconds. It is calculated by summing TrackCoverageLength with TrackGaps.
    pub track_discrete_coverage_length: f32,
    /// Query length in seconds.
    pub query_length: f32,
    /// Track length in seconds.
    pub track_length: f32,
    /// List of identified gaps in the query.
    pub query_gaps: Vec<Gap>,
    /// List of identified gaps in the track.
    pub track_gaps: Vec<Gap>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gap {
    /// Starting position of the gap in seconds.
    pub start: f32,
    /// Ending position of the gap in seconds.
    pub end: f32,
    /// Value indicating whether the gap is on the very beginning or very end.
    pub is_on_edge: bool,
    /// Gets length in seconds calculated by the difference: End - Start.
    pub length_in_seconds: f32,
}
