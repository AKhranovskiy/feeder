import os

import args
import numpy as np
import tensorflow as tf
import util
from tensorflow import keras

os.environ["TF_CPP_MIN_LOG_LEVEL"] = "3"
tf.get_logger().setLevel("ERROR")


config = args.parse_predict()

print("Load YAMNET model")
yamnet_model = tf.saved_model.load("models/yamnet")

print(f"Load {config.model_name} model")
adbanda_model = keras.models.load_model(f"models/{config.model_name}")


def filename_to_predictions(model, filename):
    audio_wav = util.load_16k_audio_wav(filename)
    _, embeddings, _ = yamnet_model(audio_wav)  # type: ignore
    predictions = model.predict(embeddings)
    return predictions


print(f"Process audio file {config.input}")

predictions = filename_to_predictions(adbanda_model, config.input)

print(list(np.argmax(predictions, axis=-1)))

infered_class = config.class_names[predictions.mean(axis=0).argmax()]
print(infered_class)
