import numpy as np
import tensorflow as tf
from tensorflow import keras

import args
import util

config = args.parse_predict()

print("Load YAMNET model")
yamnet_model = tf.saved_model.load("models/yamnet")

print(f"Load {config.model_name} model")
adbanda_model = keras.models.load_model(f"models/{config.model_name}")

print(f"Process audio file {config.input}")

audio_wav = util.load_16k_audio_wav(config.input)
_, embeddings, _ = yamnet_model(audio_wav)  # type: ignore
predictions = adbanda_model.predict(embeddings)  # type: ignore

print(list(np.argmax(predictions, axis=-1)))

infered_class = config.class_names[predictions.mean(axis=0).argmax()]
print(infered_class)
