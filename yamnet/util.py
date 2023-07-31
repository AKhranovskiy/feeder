from typing import Tuple

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


@tf.function
def filepath_to_embeddings(model, num_classes, filename, label):
    # Load 16k audio wave
    audio_wav = load_16k_audio_wav(filename)

    # Get audio embeddings & scores.
    # The embeddings are the audio features extracted using transfer learning
    _, embeddings, _ = model(audio_wav)  # type: ignore

    # Number of embeddings in order to know how many times to repeat the label
    embeddings_num = tf.shape(embeddings)[0]
    labels = tf.repeat(label, embeddings_num)

    # Using one-hot in order to use AUC
    return (embeddings, tf.one_hot(labels, num_classes))


def process_dataset(model, num_classes, batch_size, dataset):
    dataset = dataset.map(
        lambda x, y: filepath_to_embeddings(model, num_classes, x, y),
        num_parallel_calls=tf.data.experimental.AUTOTUNE,
    ).unbatch()

    return dataset.cache().batch(batch_size).prefetch(tf.data.AUTOTUNE)


def prepare_datasets(config, yamnet) -> Tuple[tf.data.Dataset, tf.data.Dataset]:
    def f(ds):
        return process_dataset(yamnet, config.num_classes, config.batch_size, ds)

    return (f(config.train_dataset), f(config.validation_dataset))
