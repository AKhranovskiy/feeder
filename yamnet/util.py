import tensorflow as tf
import tensorflow_io as tfio


@tf.function
def load_16k_audio_wav(filename):
    """
    Decorated TensorFlow function that loads an audio file from the given file
    path and returns a tensor of audio waveforms resampled to 16kHz.
    The input is a string, the path to the audio file.
    """
    # Read file content
    file_content = tf.io.read_file(filename)

    # Decode audio wave
    audio_wav, sample_rate = tf.audio.decode_wav(file_content, desired_channels=1)
    audio_wav = tf.squeeze(audio_wav, axis=-1)
    sample_rate = tf.cast(sample_rate, dtype=tf.int64)

    # Resample to 16k
    audio_wav = tfio.audio.resample(audio_wav, rate_in=sample_rate, rate_out=16000)

    return audio_wav
