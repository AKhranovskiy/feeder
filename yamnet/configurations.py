from dataclasses import dataclass
from functools import reduce
from itertools import chain

import tensorflow as tf


@dataclass
class TrainParams:
    epochs: int
    batch_size: int
    validation_ratio: float
    seed: int


DEFAULT_TRAIN_PARAMS = TrainParams(
    epochs=6, batch_size=64, validation_ratio=0.1, seed=1231239
)


class Dataset:
    def __init__(self, root_dir: str | None, seed: int):
        self._adverts = self._list_files(root_dir, seed, "advert")
        self._music = self._list_files(root_dir, seed, "music")
        self._talk = self._list_files(root_dir, seed, "talk")

    def _list_files(
        self, root_dir: str | None, seed: int, name: str
    ) -> tf.data.Dataset:
        return (
            tf.data.Dataset.list_files(
                f"{root_dir}/{name}/*.wav",
                shuffle=True,
                seed=seed,
                name=name,
            )
            if root_dir
            else tf.data.Dataset.list_files(".")
        )

    @property
    def adverts(self) -> tf.data.Dataset:
        return self._adverts

    @property
    def music(self) -> tf.data.Dataset:
        return self._music

    @property
    def talk(self) -> tf.data.Dataset:
        return self._talk


class TrainConfig(TrainParams):
    def __init__(
        self,
        params: TrainParams,
        model_name: str,
        classes: dict[str, tf.data.Dataset],
    ):
        super().__init__(
            params.epochs, params.batch_size, params.validation_ratio, params.seed
        )

        self._model_name = model_name
        self._class_names: list[str] = list(classes)

        # Build a list of labels for concatenated datasets
        # (2 Adverts, 4 Music, 1 Talk) => [0] * 5 + [1] * 7 + [2] * 9 => [0,0,1,1,1,1,2]
        labels = list(
            chain(*([i] * len(ds) for (i, ds) in enumerate(classes.values())))
        )
        labels = tf.data.Dataset.from_tensor_slices(labels)

        values = reduce(tf.data.Dataset.concatenate, classes.values())

        dataset = tf.data.Dataset.zip((values, labels))
        dataset = dataset.shuffle(len(dataset), seed=self.seed)

        split = int(len(dataset) * (1 - self.validation_ratio))
        self._train_ds = dataset.take(split)
        self._valid_ds = dataset.skip(split)

    @property
    def model_name(self) -> str:
        return self._model_name

    @property
    def class_names(self) -> list[str]:
        return self._class_names

    @property
    def num_classes(self) -> int:
        return len(self._class_names)

    @property
    def train_dataset(self) -> tf.data.Dataset:
        return self._train_ds

    @property
    def validation_dataset(self) -> tf.data.Dataset:
        return self._valid_ds


class MusicOrOtherConfig(TrainConfig):
    def __init__(
        self,
        model_name: str,
        dataset_root: str | None = None,
        params: TrainParams = DEFAULT_TRAIN_PARAMS,
    ):
        ds = Dataset(dataset_root, params.seed)
        super().__init__(
            params,
            model_name,
            classes={"music": ds.music, "other": ds.adverts.concatenate(ds.talk)},
        )


class AdvertOrTalkConfig(TrainConfig):
    def __init__(
        self,
        model_name: str,
        dataset_root: str | None = None,
        params: TrainParams = DEFAULT_TRAIN_PARAMS,
    ):
        ds = Dataset(dataset_root, params.seed)
        super().__init__(
            params, model_name, classes={"advert": ds.adverts, "talk": ds.talk}
        )


class AdvertMusicOrTalkConfig(TrainConfig):
    def __init__(
        self,
        model_name: str,
        dataset_root: str | None = None,
        params: TrainParams = DEFAULT_TRAIN_PARAMS,
    ):
        ds = Dataset(dataset_root, params.seed)
        super().__init__(
            params,
            model_name,
            classes={"advert": ds.adverts, "music": ds.music, "talk": ds.talk},
        )


@dataclass
class PredictionConfig:
    model_name: str
    class_names: list[str]
    input: str
