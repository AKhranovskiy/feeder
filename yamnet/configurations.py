from dataclasses import dataclass
from enum import Enum
from functools import reduce
from typing import Sequence

import dataset
import tensorflow as tf
from modelhp import HP_AT_BEST, HP_ATM_BEST, HP_MO_BEST, ModelHyperparameters


class ModelType(Enum):
    AMT = "amt"
    MO = "mo"
    AT = "at"


@dataclass
class ModelConfig:
    type: ModelType
    classes: list[str]
    hyperparams: ModelHyperparameters

    @property
    def name(self):
        return f"adbanda_{self.type.value}"


AMT_MODEL = ModelConfig(ModelType.AMT, ["advert", "music", "talk"], HP_ATM_BEST)
MO_MODEL = ModelConfig(ModelType.MO, ["music", "other"], HP_MO_BEST)
AT_MODEL = ModelConfig(ModelType.AT, ["advert", "talk"], HP_AT_BEST)


def combine_datasets(
    model_config: ModelConfig, dataset: dataset.Dataset
) -> tf.data.Dataset:
    num_classes = len(model_config.classes)

    def label_embeddings(embeddings: tf.data.Dataset, label: int) -> tf.data.Dataset:
        return (
            embeddings.batch(64)
            .prefetch(tf.data.AUTOTUNE)
            .map(
                lambda x: (
                    x,
                    tf.one_hot(tf.repeat(label, tf.shape(x)[0]), num_classes),
                ),
                num_parallel_calls=tf.data.AUTOTUNE,
            )
        )

    def concat(data: Sequence[tf.data.Dataset]) -> tf.data.Dataset:
        return reduce(
            tf.data.Dataset.concatenate,  # type: ignore
            (
                label_embeddings(embeddings, label)
                for (label, embeddings) in enumerate(data)
            ),
        )

    match model_config.type:
        case ModelType.AMT:
            return concat([dataset.adverts, dataset.music, dataset.talk])
        case ModelType.MO:
            return concat([dataset.music, dataset.adverts.concatenate(dataset.talk)])
        case ModelType.AT:
            return concat([dataset.adverts, dataset.talk])
        case _:
            assert False, "Should never get here"


@dataclass
class TrainParams:
    epochs: int
    batch_size: int
    validation_ratio: float
    seed: int


DEFAULT_TRAIN_PARAMS = TrainParams(
    epochs=6, batch_size=64, validation_ratio=0.1, seed=1231239
)


class TrainConfig:
    def __init__(
        self,
        model_config: ModelConfig,
        data_dir: str,
        train_params: TrainParams = DEFAULT_TRAIN_PARAMS,
    ):
        self._train_params = train_params
        self._model_config = model_config

        full_ds = combine_datasets(model_config, dataset.Dataset(data_dir))
        full_ds = full_ds.shuffle(len(full_ds), seed=train_params.seed)

        split = int(len(full_ds) * (1 - train_params.validation_ratio))
        self._train_ds = full_ds.take(split)
        self._valid_ds = full_ds.skip(split)

    @property
    def model_name(self) -> str:
        return self._model_config.name

    @property
    def class_names(self) -> list[str]:
        return self._model_config.classes

    @property
    def num_classes(self) -> int:
        return len(self._model_config.classes)

    @property
    def train_dataset(self) -> tf.data.Dataset:
        return self._train_ds

    @property
    def validation_dataset(self) -> tf.data.Dataset:
        return self._valid_ds

    @property
    def seed(self) -> int:
        return self._train_params.seed

    @property
    def epochs(self) -> int:
        return self._train_params.epochs

    @property
    def hyperparameters(self) -> ModelHyperparameters:
        return self._model_config.hyperparams


@dataclass
class PredictionConfig:
    model_config: ModelConfig
    input: str

    @property
    def model_name(self) -> str:
        return self.model_config.name

    @property
    def class_names(self) -> list[str]:
        return self.model_config.classes
