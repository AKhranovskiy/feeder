import keras_tuner
from configurations import ModelConfig, TrainConfig
from modelhp import LayerHyperparameters, ModelHyperparameters
from tensorflow import keras


def build_model(config: TrainConfig):
    hp = config.hyperparameters
    print(f"Building model: {hp}")

    inputs = keras.layers.Input(shape=(1024), name="embedding")
    x = inputs

    for i in range(len(hp.layers)):
        layer = hp.layers[i]

        x = keras.layers.Dense(
            layer.units, activation=hp.layer_activation, name=f"dense_{i}"
        )(x)

        if layer.dropout is not None:
            x = keras.layers.Dropout(layer.dropout, name=f"dropout_{i}")(x)

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


def build_hypertuner(config: TrainConfig):
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
            TrainConfig(
                model_config=ModelConfig(
                    type=config.model_type,
                    classes=config.class_names,
                    hyperparams=ModelHyperparameters(
                        layers,
                        layer_activation,  # type: ignore
                        output_activation,  # type: ignore
                        learning_rate,  # type: ignore
                    ),
                ),
                data_dir=config.data_dir,
                train_params=config.train_params,
            )
        )

    return keras_tuner.BayesianOptimization(
        build_hp,
        objective="val_accuracy",
        max_trials=10,
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
