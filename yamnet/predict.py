import os
import sys
import numpy as np
import tensorflow as tf
import tensorflow_io as tfio
from tensorflow import keras

os.environ['TF_CPP_MIN_LOG_LEVEL'] = '3'
tf.get_logger().setLevel('ERROR')

class_names = ['advert', 'music', 'talk']

yamnet_model = tf.saved_model.load('models/yamnet')
adbanda_model = keras.models.load_model('models/adbanda_2')

@tf.function
def load_16k_audio_wav(filename):
    # Read file content
    file_content = tf.io.read_file(filename)

    # Decode audio wave
    audio_wav, sample_rate = tf.audio.decode_wav(file_content, desired_channels=1)
    audio_wav = tf.squeeze(audio_wav, axis=-1)
    sample_rate = tf.cast(sample_rate, dtype=tf.int64)

    # Resample to 16k
    audio_wav = tfio.audio.resample(audio_wav, rate_in=sample_rate, rate_out=16000)

    return audio_wav

def filename_to_predictions(model, filename):
    audio_wav = load_16k_audio_wav(filename)
    _, embeddings, _ = yamnet_model(audio_wav) # type: ignore
    predictions = model.predict(embeddings)
    return predictions


predictions = filename_to_predictions(adbanda_model, sys.argv[1])

print(list(np.argmax(predictions, axis=-1)))

infered_class = class_names[predictions.mean(axis=0).argmax()]
print(infered_class)
