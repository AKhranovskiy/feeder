import os

# import config
import tensorflow as tf
from tensorflow import keras

import args
import util

os.environ["TF_CPP_MIN_LOG_LEVEL"] = "3"
tf.get_logger().setLevel("ERROR")

config = args.parse_train()

# Set all random seeds in order to get reproducible results
keras.utils.set_random_seed(config.seed)

print(f"Training model {config.model_name}")
print("Loading dataset")

train_ds = config.train_dataset.cache()
valid_ds = config.validation_dataset.cache()

print(
    f"We have {len(train_ds)} training samples \
      & {len(valid_ds)} validation ones"
)

yamnet_model = tf.saved_model.load("models/yamnet")


@tf.function
def filepath_to_embeddings(filename, label):
    # Load 16k audio wave
    audio_wav = util.load_16k_audio_wav(filename)

    # Get audio embeddings & scores.
    # The embeddings are the audio features extracted using transfer learning
    _, embeddings, _ = yamnet_model(audio_wav)  # type: ignore

    # Number of embeddings in order to know how many times to repeat the label
    embeddings_num = tf.shape(embeddings)[0]
    labels = tf.repeat(label, embeddings_num)

    # Using one-hot in order to use AUC
    return (embeddings, tf.one_hot(labels, len(config.class_names)))


def process_dataset(dataset):
    dataset = dataset.map(
        lambda x, y: filepath_to_embeddings(x, y),
        num_parallel_calls=tf.data.experimental.AUTOTUNE,
    ).unbatch()

    return dataset.cache().batch(config.batch_size).prefetch(tf.data.AUTOTUNE)


print("Processing train dataset")
train_ds = process_dataset(train_ds)

print("Processing validation dataset")
valid_ds = process_dataset(valid_ds)

# This step takes a lot of time because it eagerly computes TF graph,
# converting audio samples to embeddings.
# print("Calculate class weights")
# class_counts = train_ds.reduce(
#     tf.zeros(shape=(len(config.class_names),), dtype=tf.int32),
#     lambda acc, item: acc
#     + tf.math.bincount(
#         tf.cast(tf.math.argmax(item[1], axis=1), tf.int32),
#         minlength=len(config.class_names),
#     ),
# )

# class_weight = {
#     i: float(tf.math.reduce_sum(class_counts).numpy() / class_counts[i].numpy())
#     for i in range(len(class_counts))
# }

# print({config.class_names[k]: class_weight[k] for k in class_weight})

keras.backend.clear_session()


def build_and_compile_model():
    inputs = keras.layers.Input(shape=(1024), name="embedding")

    x = keras.layers.Dense(256, activation="relu", name="dense_1")(inputs)
    x = keras.layers.Dropout(0.15, name="dropout_1")(x)

    x = keras.layers.Dense(384, activation="relu", name="dense_2")(x)
    x = keras.layers.Dropout(0.2, name="dropout_2")(x)

    x = keras.layers.Dense(192, activation="relu", name="dense_3")(x)
    x = keras.layers.Dropout(0.25, name="dropout_3")(x)

    x = keras.layers.Dense(384, activation="relu", name="dense_4")(x)
    x = keras.layers.Dropout(0.2, name="dropout_4")(x)

    outputs = keras.layers.Dense(
        len(config.class_names), activation="softmax", name="ouput"
    )(x)

    model = keras.Model(inputs=inputs, outputs=outputs, name=config.model_name)

    model.compile(
        optimizer=keras.optimizers.Adam(learning_rate=0.00002),
        loss=keras.losses.CategoricalCrossentropy(),
        metrics=["accuracy", keras.metrics.AUC(name="auc")],
    )

    return model


model = build_and_compile_model()
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
