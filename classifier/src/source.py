import os

import tensorflow as tf
from tensorflow import keras

SEED = 1234567

# Set all random seeds in order to get reproducible results
keras.utils.set_random_seed(SEED)

os.environ["TF_CPP_MIN_LOG_LEVEL"] = "3"
tf.get_logger().setLevel("ERROR")

YAMNET = tf.saved_model.load("models/yamnet")


def num_gpus():
    return len(tf.config.list_physical_devices("GPU"))


def load_model(name):
    return keras.models.load_model(name)


def predict(model, data):
    _, embeddings, _ = YAMNET(data)  # type: ignore
    return model.predict(embeddings, verbose=0)
