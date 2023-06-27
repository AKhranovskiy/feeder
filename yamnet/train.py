import os
import tensorflow as tf
import tensorflow_io as tfio
from tensorflow import keras

import config
import util

os.environ["TF_CPP_MIN_LOG_LEVEL"] = "3"
tf.get_logger().setLevel("ERROR")

# Set all random seeds in order to get reproducible results
keras.utils.set_random_seed(config.SEED)

print(f"Training model {config.MODEL_NAME}")
print("Loading dataset")

# The current dataset contains 15-20sec ads, and 30sec music and talk samples.
# I'm trying to balance the dataset, so music and talks take half of number of ads samples.
advert = tf.data.Dataset.list_files(
    "dataset/advertisement/*.wav", shuffle=True, seed=config.SEED, name="Advert"
).take(
    config.DATASET_LIMIT
)  # type: ignore

music = tf.data.Dataset.list_files(
    "dataset/music/*.wav", shuffle=True, seed=config.SEED, name="Music"
).take(
    len(advert) // 2
)  # type: ignore

talk = tf.data.Dataset.list_files(
    "dataset/talk/*.wav", shuffle=True, seed=config.SEED, name="Talk"
).take(
    len(advert) // 2
)  # type: ignore

labels = tf.data.Dataset.from_tensor_slices(
    [config.CLASS_ID[0]] * len(advert)
    + [config.CLASS_ID[1]] * len(music)
    + [config.CLASS_ID[2]] * len(talk)
)
files = advert.concatenate(music).concatenate(talk)  # type: ignore

dataset = tf.data.Dataset.zip((files, labels))
dataset = dataset.shuffle(len(dataset), seed=config.SEED)

split = int(len(dataset) * (1 - config.VALIDATION_RATIO))
train_ds = dataset.take(split).cache()
valid_ds = dataset.skip(split).cache()

print(f"We have {len(train_ds)} training samples & {len(valid_ds)} validation ones")

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
    return (embeddings, tf.one_hot(labels, len(config.CLASS_NAMES)))


def process_dataset(dataset):
    dataset = dataset.map(
        lambda x, y: filepath_to_embeddings(x, y),
        num_parallel_calls=tf.data.experimental.AUTOTUNE,
    ).unbatch()

    return dataset.cache().batch(config.BATCH_SIZE).prefetch(tf.data.AUTOTUNE)


print(f"Processing train dataset")
train_ds = process_dataset(train_ds)

print(f"Processing validation dataset")
valid_ds = process_dataset(valid_ds)

# This step takes a lot of time because it eagerly computes TF graph, converting audio samples to embeddings.
print(f"Calculate class weights")
class_counts = train_ds.reduce(
    tf.zeros(shape=(len(config.CLASS_NAMES),), dtype=tf.int32),
    lambda acc, item: acc
    + tf.math.bincount(
        tf.cast(tf.math.argmax(item[1], axis=1), tf.int32),
        minlength=len(config.CLASS_NAMES),
    ),
)

class_weight = {
    i: tf.math.reduce_sum(class_counts).numpy() / class_counts[i].numpy()
    for i in range(len(class_counts))
}

print({config.CLASS_NAMES[k]: class_weight[k] for k in class_weight})

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
        len(config.CLASS_NAMES), activation="softmax", name="ouput"
    )(x)

    model = keras.Model(inputs=inputs, outputs=outputs, name=config.MODEL_NAME)

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
    epochs=config.EPOCHS,
    validation_data=valid_ds,
    callbacks=callbacks,
    class_weight=class_weight,
    verbose=1,  # type: ignore
)

model.save(f"models/{config.MODEL_NAME}")
