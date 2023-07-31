import args
import tensorflow as tf
from tensorflow import keras

from tools import model_hypertuner  # type: ignore

config = args.parse_train()

# Set all random seeds in order to get reproducible results
keras.utils.set_random_seed(config.seed)

print(f"Hypertune model {config.model_name}")

print("Loading YAMNET model")
yamnet_model = tf.saved_model.load("models/yamnet")

keras.backend.clear_session()

tuner = model_hypertuner(config)
tuner.search_space_summary()


class LowValAccuracyCallback(tf.keras.callbacks.Callback):
    def __init__(self, min_val_accuracy: float = 0.80):
        self.min_val_accuracy = min_val_accuracy

    def on_epoch_end(self, epoch, logs=None):
        logs = logs or {}
        acc = logs.get("val_accuracy")
        if acc is None:
            return

        if acc >= self.min_val_accuracy:
            return

        print(
            f"\n\nval_accuracy={acc:.3} is below threshold {self.min_val_accuracy:.3}"
            + ", terminating training\n"
        )
        self.model.stop_training = True  # type: ignore


# Half of dataset should be enough to estimate
tuner.search(
    config.train_dataset.take(int(0.5 * len(config.train_dataset))),
    epochs=5,
    validation_data=config.validation_dataset,
    verbose=1,
    callbacks=[LowValAccuracyCallback()],
)
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
