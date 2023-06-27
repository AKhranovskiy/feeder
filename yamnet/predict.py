import os
import sys
import numpy as np
import tensorflow as tf
import tensorflow_io as tfio
from tensorflow import keras

import config
import util

os.environ["TF_CPP_MIN_LOG_LEVEL"] = "3"
tf.get_logger().setLevel("ERROR")


print("Load YAMNET model")
yamnet_model = tf.saved_model.load("models/yamnet")

print(f"Load {config.MODEL_NAME} model")
adbanda_model = keras.models.load_model(f"models/{config.MODEL_NAME}")


def filename_to_predictions(model, filename):
    audio_wav = util.load_16k_audio_wav(filename)
    _, embeddings, _ = yamnet_model(audio_wav)  # type: ignore
    predictions = model.predict(embeddings)
    return predictions


print(f"Process audio file {sys.argv[1]}")

predictions = filename_to_predictions(adbanda_model, sys.argv[1])

print(list(np.argmax(predictions, axis=-1)))

infered_class = config.CLASS_NAMES[predictions.mean(axis=0).argmax()]
print(infered_class)
