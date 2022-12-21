use std::io::Cursor;

use std::str::FromStr;

use ac_ffmpeg::codec::audio::{AudioDecoder, AudioResampler, ChannelLayout, SampleFormat};
use ac_ffmpeg::codec::Decoder;
use ac_ffmpeg::format::demuxer::Demuxer;
use ac_ffmpeg::format::io::IO;

use bytemuck::cast_slice;


pub async fn audio_to_pcm_s16le(input: Vec<u8>) -> anyhow::Result<Vec<i16>> {
    let io = IO::from_seekable_read_stream(Cursor::new(input));

    let mut demuxer = Demuxer::builder()
        .set_option("loglevel", "fatal")
        .build(io)?
        .find_stream_info(None)
        .map_err(|(_, err)| err)?;

    let mut decoder = AudioDecoder::from_stream(&demuxer.streams()[0])?
        .set_option("loglevel", "fatal")
        .build()?;

    let codec = demuxer.streams()[0].codec_parameters();
    let params = codec.as_audio_codec_parameters().unwrap();

    let mut resampler = AudioResampler::builder()
        .source_sample_rate(params.sample_rate())
        .source_sample_format(params.sample_format())
        .source_channel_layout(params.channel_layout())
        .target_sample_rate(22050)
        .target_sample_format(SampleFormat::from_str("s16").expect("Sample format for analysis"))
        .target_channel_layout(ChannelLayout::from_channels(1).expect("Mono channel layout"))
        .build()?;

    let mut output = Vec::<u8>::new();

    while let Some(packet) = demuxer.take()? {
        decoder.push(packet)?;
        while let Some(frame) = decoder.take()? {
            resampler.push(frame)?;
            while let Some(frame) = resampler.take()? {
                output.extend_from_slice(&frame.planes()[0].data());
            }
        }
    }

    // let ffmpeg_path = env!("FFMPEG_PATH");
    //
    // let mut proc = Command::new("ffmpeg")
    //     .env("PATH", ffmpeg_path)
    //     .stdin(Stdio::piped())
    //     .stdout(Stdio::piped())
    //     .args(
    //         format!("-i pipe:0 -acodec pcm_s16le -ar {SAMPLE_RATE} -ac 1 -f wav -v fatal pipe:1")
    //             .split_ascii_whitespace(),
    //     )
    //     .spawn()?;
    //
    // // println!("Spawned ffmpeg");
    //
    // let mut stdin = proc
    //     .stdin
    //     .take()
    //     .ok_or_else(|| anyhow!("failed to get stdin"))?;
    //
    // // Spawn separate task to not block the current context on writing/reading.
    // tokio::spawn(async move {
    //     stdin
    //         .write_all(&input)
    //         .await
    //         .expect("Failed to write content");
    //     // println!("Written {}kb", data.len() / 1024);
    // });
    //
    // let output = proc.wait_with_output().await?.stdout;
    // println!("Read {}kb", output.len() / 1024);

    let data = cast_slice::<u8, i16>(output.as_slice()).to_vec();

    Ok(data)
}
