import os

import tensorflow as tf
from tensorflow import keras

import args
import tools
import util

os.environ["TF_CPP_MIN_LOG_LEVEL"] = "3"
tf.get_logger().setLevel("ERROR")

config = args.parse_train()

# Set all random seeds in order to get reproducible results
keras.utils.set_random_seed(config.seed)

print(f"Training model {config.model_name}")

print("Loading YAMNET model")
yamnet_model = tf.saved_model.load("models/yamnet")

print("Prepare dataset")

print(f"Training dataset size: {len(config.train_dataset)}")
print(f"Validation dataset size: {len(config.validation_dataset)}")

(train_ds, valid_ds) = util.prepare_datasets(config, yamnet_model)

keras.backend.clear_session()

model = tools.build_model(config, tools.HP_BEST)

model.summary()

early_stopping_cb = keras.callbacks.EarlyStopping(
    monitor="val_auc", patience=10, restore_best_weights=True
)

checkpoint_filepath = "/tmp/checkpoint/adbanda"
model_checkpoint_cb = keras.callbacks.ModelCheckpoint(
    checkpoint_filepath, monitor="val_auc", save_best_only=True
)

tensorboard_cb = keras.callbacks.TensorBoard(
    os.path.join(os.curdir, "logs", model.name)
)

callbacks = [early_stopping_cb, model_checkpoint_cb, tensorboard_cb]

history = model.fit(
    train_ds,
    epochs=config.epochs,
    validation_data=valid_ds,
    callbacks=callbacks,
    # class_weight=class_weight,
    verbose=1,  # type: ignore
)

model.save(f"models/{config.model_name}")
