from dataclasses import dataclass


@dataclass
class LayerHyperparameters:
    units: int
    dropout: float | None = None

    def __str__(self) -> str:
        dropout = "" if self.dropout is None else f", dropout={self.dropout:.3e}"
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


HP_ATM_ORIG = ModelHyperparameters(
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

HP_ATM_BEST = ModelHyperparameters(
    layers=[
        LayerHyperparameters(units=64, dropout=0.069),
        LayerHyperparameters(units=640, dropout=0.009),
        LayerHyperparameters(units=512, dropout=0.272),
    ],
    layer_activation="relu",
    output_activation="softmax",
    learning_rate=0.0003,
)

HP_ATM_SECOND = ModelHyperparameters(
    layers=[
        LayerHyperparameters(units=896, dropout=0.083),
        LayerHyperparameters(units=704, dropout=0.113),
        LayerHyperparameters(units=1024, dropout=0.056),
        LayerHyperparameters(units=512, dropout=0.074),
        LayerHyperparameters(units=32),
    ],
    layer_activation="relu",
    output_activation="softmax",
    learning_rate=3e-05,
)

HP_MO_BEST = ModelHyperparameters(
    layers=[LayerHyperparameters(units=320, dropout=8.106e-02)],
    layer_activation="tanh",
    output_activation="sigmoid",
    learning_rate=2.782e-04,
)

HP_AT_BEST = ModelHyperparameters(
    layers=[LayerHyperparameters(units=128, dropout=2.522e-01)],
    layer_activation="relu",
    output_activation="softmax",
    learning_rate=3.524e-04,
)
