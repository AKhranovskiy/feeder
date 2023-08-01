import os

import args
from factory import build_model
from tensorflow import keras

config = args.parse_train()

keras.utils.set_random_seed(config.seed)

print(f"Training model {config.model_name}")

keras.backend.clear_session()

model = build_model(config)

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

class_weight = config.class_weight
print({config.class_names[k]: class_weight[k] for k in class_weight})

history = model.fit(
    config.train_dataset,
    epochs=config.epochs,
    validation_data=config.validation_dataset,
    callbacks=callbacks,
    class_weight=class_weight,
    verbose=1,  # type: ignore
)

model.save(f"models/{config.model_name}")
