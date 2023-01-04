use std::io::{Read, Seek};

use bytemuck::cast_slice;

mod demuxer;
pub use demuxer::Demuxer;

mod decoder;
pub use decoder::Decoder;

mod resampler;
pub use resampler::{CodecParams, ResamplingDecoder, SampleFormat};

pub fn decode<RS>(input: RS) -> anyhow::Result<Vec<i16>>
where
    RS: Read + Seek,
{
    resample(input, CodecParams::new(22050, SampleFormat::S16, 1))
        .map(|data| cast_slice::<u8, i16>(data.as_slice()).to_vec())
}

pub fn resample<RS>(input: RS, target: CodecParams) -> anyhow::Result<Vec<u8>>
where
    RS: Read + Seek,
{
    let decoder = Decoder::try_from(input)?.resample(target);

    let mut output = vec![];

    for frame in decoder {
        for plane in frame?.planes().iter() {
            output.extend_from_slice(plane.data());
        }
    }

    Ok(output)
}
