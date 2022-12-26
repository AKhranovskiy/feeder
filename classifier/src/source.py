from sklearn.model_selection import train_test_split
from tensorflow import keras
from keras import Sequential
from keras import layers

INPUT_SHAPE = [150, 39, 1]
CLASSES = 3
TEST_SIZE = 0.25
VALIDATION_SIZE = 0.2
EPOCHS = 100
BATCH = 32


def define_model():
    model = Sequential(
        [
            layers.Conv2D(
                32,
                (3, 3),
                activation="relu",
                padding="valid",
                name="model_in",
                input_shape=INPUT_SHAPE,
            ),
            layers.MaxPooling2D(2, padding="same"),
            layers.Conv2D(128, (3, 3), activation="relu", padding="valid"),
            layers.MaxPooling2D(2, padding="same"),
            layers.Dropout(0.3),
            layers.Conv2D(128, (3, 3), activation="relu", padding="valid"),
            layers.MaxPooling2D(2, padding="same"),
            layers.Dropout(0.3),
            layers.GlobalAveragePooling2D(),
            layers.Dense(512, activation="relu"),
            layers.Dense(CLASSES, activation="softmax", name="model_out"),
        ]
    )
    model.compile(loss="binary_crossentropy", optimizer="adam", metrics="acc")
    return model


def load_model(name):
    return keras.models.load_model(name)


def save_model(model, name):
    model.save(name)


def train_model(model, data, labels):
    labels = keras.utils.to_categorical(labels, num_classes=CLASSES)

    x_train, x_val, y_train, y_val = train_test_split(
        data, labels, test_size=VALIDATION_SIZE
    )

    model.fit(
        x_train,
        y_train,
        validation_data=(x_val, y_val),
        epochs=EPOCHS,
        verbose=2,
        batch_size=BATCH,
    )

    return model


def predict(model, data):
    return model(data, training=False).numpy()
