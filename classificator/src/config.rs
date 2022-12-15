pub const SAMPLE_RATE: usize = 22050;

// FFT 512 is recommended by librosa for speech processing
// FFT 2048 is used by PP CNN.
pub const MFCC_FRAME_SIZE: usize = 441; // 20ms
pub const MFCC_HOP_LENGTH: usize = 220; // 10ms
pub const MFCC_N_FILTERS: usize = 40;
pub const MFCC_N_COEFFS: usize = 39;

pub const CLASSIFICATION_SEGMENT_LENGTH: usize = 1 * SAMPLE_RATE;
