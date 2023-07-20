import os

import tensorflow as tf
from tensorflow import keras

import args
import util
from tools import model_hypertuner  # type: ignore

os.environ["TF_CPP_MIN_LOG_LEVEL"] = "3"
tf.get_logger().setLevel("ERROR")

config = args.parse_train()

# Set all random seeds in order to get reproducible results
keras.utils.set_random_seed(config.seed)

print(f"Hypertune model {config.model_name}")

print("Loading YAMNET model")
yamnet_model = tf.saved_model.load("models/yamnet")

print("Prepare dataset")

print(f"Training dataset size: {len(config.train_dataset)}")
print(f"Validation dataset size: {len(config.validation_dataset)}")

(train_ds, valid_ds) = util.prepare_datasets(config, yamnet_model)

keras.backend.clear_session()

tuner = model_hypertuner(config)
tuner.search_space_summary()

tuner.search(train_ds, epochs=5, validation_data=valid_ds, verbose=1)
tuner.results_summary(num_trials=2)

best_models = tuner.get_best_models(num_models=2)

print("Best model #1")
best_model = best_models[0]
best_model.build(input_shape=(1024))
best_model.summary()

print("Best model #2")
best_model = best_models[1]
best_model.build(input_shape=(1024))
best_model.summary()
