from dataclasses import dataclass

import keras_tuner
from tensorflow import keras

from configurations import TrainConfig


@dataclass
class LayerHyperparameters:
    units: int
    dropout: float | None = None

    def __str__(self) -> str:
        dropout = "" if self.dropout is None else f", dropout={self.dropout}"
        return f"units={self.units}{dropout}"


@dataclass
class ModelHyperparameters:
    layers: list[LayerHyperparameters]
    layer_activation: str
    output_activation: str
    learning_rate: float

    def __str__(self) -> str:
        layers = "\n".join(str(x) for x in self.layers)
        return (
            f"{layers}\n"
            + f"layer_activation={self.layer_activation}, "
            + f"output_activation={self.output_activation}, "
            + f"learning_rate={self.learning_rate:.3e}"
        )


def build_model(config: TrainConfig, hp: ModelHyperparameters):
    print("Building model:")
    print(hp)

    inputs = keras.layers.Input(shape=(1024), name="embedding")
    x = inputs

    for i in range(len(hp.layers)):
        layer = hp.layers[i]

        x = keras.layers.Dense(
            layer.units, activation=hp.layer_activation, name=f"dense_{i}"
        )(x)

        if layer.dropout is not None:
            x = keras.layers.Dropout(layer.dropout, name=f"dropout_{i}")(x)

    # x = keras.layers.Dense(256, activation="relu", name="dense_1")(inputs)
    # x = keras.layers.Dropout(0.15, name="dropout_1")(x)

    # x = keras.layers.Dense(384, activation="relu", name="dense_2")(x)
    # x = keras.layers.Dropout(0.2, name="dropout_2")(x)

    # x = keras.layers.Dense(192, activation="relu", name="dense_3")(x)
    # x = keras.layers.Dropout(0.25, name="dropout_3")(x)

    # x = keras.layers.Dense(384, activation="relu", name="dense_4")(x)
    # x = keras.layers.Dropout(0.2, name="dropout_4")(x)

    outputs = keras.layers.Dense(
        config.num_classes, activation=hp.output_activation, name="ouput"
    )(x)

    model = keras.Model(inputs=inputs, outputs=outputs, name=config.model_name)

    model.compile(
        optimizer=keras.optimizers.Adam(learning_rate=hp.learning_rate),
        loss=keras.losses.CategoricalCrossentropy(),
        metrics=["accuracy", keras.metrics.AUC(name="auc")],
    )

    print(model.summary())

    return model


ACTIVATIONS = [
    "relu",
    "tanh",
    "elu",
    "mish",
    "selu",
    "sigmoid",
    "softmax",
    "softplus",
    "softsign",
    "swish",
]


def model_hypertuner(config: TrainConfig):
    def build_hp(hp: keras_tuner.HyperParameters) -> keras.Model:
        layers = []
        for i in range(hp.Int("num_layers", 1, 5)):  # type: ignore
            units = hp.Int(f"units_{i}", min_value=32, max_value=1024, step=32)
            dropout = hp.Float(f"dropout_{i}", min_value=0.0, max_value=0.3)

            layers.append(
                LayerHyperparameters(
                    units,  # type: ignore
                    dropout,  # type: ignore
                )
            )

        layer_activation = hp.Choice(
            "layer_activation", ["relu", "tanh", "elu", "sigmoid", "selu"]
        )
        output_activation = hp.Choice("output_activation", ["softmax", "sigmoid"])
        learning_rate = hp.Float("lr", min_value=1e-5, max_value=1e-2, sampling="log")

        return build_model(
            config,
            ModelHyperparameters(
                layers,
                layer_activation,  # type: ignore
                output_activation,  # type: ignore
                learning_rate,  # type: ignore
            ),
        )

    return keras_tuner.BayesianOptimization(
        build_hp,
        objective="val_accuracy",
        max_trials=20,
        executions_per_trial=3,
        overwrite=True,
        directory=f"/tmp/keras-tuner/{config.model_name}",
        project_name=config.model_name,
    )


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

HP_ORIG = ModelHyperparameters(
    [
        LayerHyperparameters(units=256, dropout=0.15),
        LayerHyperparameters(units=384, dropout=0.20),
        LayerHyperparameters(units=192, dropout=0.25),
        LayerHyperparameters(units=384, dropout=0.20),
    ],
    layer_activation="relu",
    output_activation="softmax",
    learning_rate=2e-5,
)

"""
units=64, droupout=0.06936726288431967
units=640, droupout=0.009831974852189196
units=512, droupout=0.2722492738414498
layer_activation=relu, output_activation=softmax, learning_rate=0.00030162274543822453
"""
"""
units=896, droupout=0.0830980980088495
units=704, droupout=0.1126367950060339
units=1024, droupout=0.05630244804326629
units=512, droupout=0.0742384384236275
units=32, droupout=0.0
layer_activation=relu, output_activation=softmax, learning_rate=3.063028198164177e-05

"""
HP_BEST_ATM = ModelHyperparameters(
    layers=[
        LayerHyperparameters(units=64, dropout=0.069),
        LayerHyperparameters(units=640, dropout=0.009),
        LayerHyperparameters(units=512, dropout=0.272),
    ],
    layer_activation="relu",
    output_activation="softmax",
    learning_rate=0.0003,
)
