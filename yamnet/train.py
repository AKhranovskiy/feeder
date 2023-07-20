import os

import tensorflow as tf
from tensorflow import keras

import args
import tools

config = args.parse_train()

keras.utils.set_random_seed(config.seed)

print(f"Training model {config.model_name}")

print("Loading YAMNET model")
yamnet_model = tf.saved_model.load("models/yamnet")

keras.backend.clear_session()

model = tools.build_model(config, tools.HP_BEST_ATM)

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

print(config.train_dataset)
# <_TakeDataset element_spec=(TensorSpec(shape=(1024,), dtype=tf.float32, name=None), TensorSpec(shape=(1024, 3), dtype=tf.float32, name=None))>

exit(0)
history = model.fit(
    config.train_dataset,
    epochs=config.epochs,
    validation_data=config.validation_dataset,
    callbacks=callbacks,
    # class_weight=class_weight,
    verbose=1,  # type: ignore
)

model.save(f"models/{config.model_name}")
