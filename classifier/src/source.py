import os
import tensorflow as tf
from tensorflow import keras


MODEL_NAME = 'adbanda'
CLASS_NAMES = ['advert', 'music']

SEED = 1234567
EPOCHS = 10
BATCH_SIZE = 64
VALIDATION_RATIO = 0.1
LEARNING_RATE = 0.00002

# Location where the dataset will be downloaded.
# By default (None), keras.utils.get_file will use ~/.keras/ as the CACHE_DIR
CACHE_DIR = None

# Set all random seeds in order to get reproducible results
keras.utils.set_random_seed(SEED)

os.environ['TF_CPP_MIN_LOG_LEVEL'] = '3'
tf.get_logger().setLevel('ERROR')

YAMNET = tf.saved_model.load('models/yamnet')

def define_model():
    inputs = keras.layers.Input(shape=(1024), name='embedding')

    x = keras.layers.Dense(256, activation='relu', name='dense_1')(inputs)
    x = keras.layers.Dropout(0.15, name='dropout_1')(x)

    x = keras.layers.Dense(384, activation='relu', name='dense_2')(x)
    x = keras.layers.Dropout(0.2, name='dropout_2')(x)

    x = keras.layers.Dense(192, activation='relu', name='dense_3')(x)
    x = keras.layers.Dropout(0.25, name='dropout_3')(x)

    x = keras.layers.Dense(384, activation='relu', name='dense_4')(x)
    x = keras.layers.Dropout(0.2, name='dropout_4')(x)

    outputs = keras.layers.Dense(
        len(CLASS_NAMES),
        activation='softmax',
        name='ouput'
    )(x)

    model = keras.Model(inputs=inputs, outputs=outputs, name=MODEL_NAME)

    model.compile(
        optimizer=keras.optimizers.Adam(learning_rate=LEARNING_RATE),
        loss=keras.losses.CategoricalCrossentropy(),
        metrics=['accuracy', keras.metrics.AUC(name='auc')],
    )

    return model


def load_model(name):
    return keras.models.load_model(name)


def save_model(model, name):
    model.save(name)


def prepare_dataset(data, labels):
    data_ds = tf.data.Dataset.from_tensor_slices(data)
    labels_ds = tf.data.Dataset.from_tensor_slices(labels)

    dataset = tf.data.Dataset.zip((data_ds, labels_ds))
    dataset = dataset.shuffle(len(dataset), seed=SEED)

    split = int(len(dataset) * (1 - VALIDATION_RATIO))
    train_ds = dataset.take(split)
    valid_ds = dataset.skip(split)

    train_ds = process_dataset(train_ds)
    valid_ds = process_dataset(valid_ds)

    return (train_ds, valid_ds)

def process_dataset(dataset):
    dataset = dataset.map(
        lambda x, y: get_embeddings(x, y),
        num_parallel_calls=tf.data.experimental.AUTOTUNE,
    ).unbatch()

    return dataset.cache().batch(BATCH_SIZE).prefetch(tf.data.AUTOTUNE)

def get_embeddings(audio, label):
    # Get audio embeddings & scores.
    # The embeddings are the audio features extracted using transfer learning
    _, embeddings, _ = YAMNET(audio) # type: ignore

    # Number of embeddings in order to know how many times to repeat the label
    embeddings_num = tf.shape(embeddings)[0]
    labels = tf.repeat(label, embeddings_num)

    # Using one-hot in order to use AUC
    return (embeddings, tf.one_hot(labels, len(CLASS_NAMES)))

def train_model(model, data, labels, epochs=EPOCHS, batch=BATCH_SIZE):
    train_ds, valid_ds = prepare_dataset(data, labels)

    early_stopping_cb = keras.callbacks.EarlyStopping(
        monitor='val_auc', patience=10, restore_best_weights=True
    )

    checkpoint_filepath = f'/tmp/checkpoint/{MODEL_NAME}'
    model_checkpoint_cb = keras.callbacks.ModelCheckpoint(
        checkpoint_filepath, monitor='val_auc', save_best_only=True
    )

    tensorboard_cb = keras.callbacks.TensorBoard(
        os.path.join(os.curdir, 'logs', model.name)
    )

    callbacks = [early_stopping_cb, model_checkpoint_cb, tensorboard_cb]

    model.fit(
        train_ds,
        epochs=EPOCHS,
        validation_data=valid_ds,
        callbacks=callbacks,
        verbose=1, # type: ignore
    )

    model.load_weights(checkpoint_filepath)

    return model

def predict(model, data):
    _, embeddings, _ = YAMNET(data) # type: ignore
    return model.predict(embeddings, verbose=0)

# print(list(np.argmax(predictions, axis=-1)))
# infered_class = CLASS_NAMES[predictions.mean(axis=0).argmax()]
