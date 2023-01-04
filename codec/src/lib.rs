use std::io::{Read, Seek};

use bytemuck::cast_slice;

mod demuxer;
mod resampler;
pub use demuxer::Demuxer;

mod decoder;
pub use decoder::Decoder;

use self::resampler::{CodecParams, SampleFormat};

pub fn decode<RS>(input: RS) -> anyhow::Result<Vec<i16>>
where
    RS: Read + Seek,
{
    let decoder = Decoder::try_from(input)?;
    let params = decoder.audio_codec_parameters().unwrap();

    let mut resampler =
        resampler::Resampler::new(params.into(), CodecParams::new(22050, SampleFormat::S16, 1));

    let mut output = vec![];

    for frame in decoder {
        for frame in resampler.push(frame?)? {
            output.extend_from_slice(frame?.planes()[0].data());
        }
    }

    Ok(cast_slice::<u8, i16>(output.as_slice()).to_vec())
}
