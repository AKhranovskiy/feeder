import os

import tensorflow as tf


def load_embeddings(path) -> tf.data.Dataset:
    print(f"Loading {path}")
    return tf.data.Dataset.load(path, compression="GZIP")  # type: ignore


class Dataset:
    def __init__(self, root_dir: str):
        self._adverts = load_embeddings(os.path.join(root_dir, "advert.embeddings"))
        self._music = load_embeddings(os.path.join(root_dir, "music.embeddings"))
        self._talk = load_embeddings(os.path.join(root_dir, "talk.embeddings"))

    def __getitem__(self, name: str) -> tf.data.Dataset:
        if name == "advert":
            return self._adverts
        elif name == "music":
            return self._music
        elif name == "talk":
            return self._talk
        else:
            assert False, f"Unknown name: {name}"

    @property
    def adverts(self) -> tf.data.Dataset:
        return self._adverts

    @property
    def music(self) -> tf.data.Dataset:
        return self._music

    @property
    def talk(self) -> tf.data.Dataset:
        return self._talk


if __name__ == "__main__":
    d = Dataset("./dataset_full")
    print(f"Adverts: {len(d.adverts)}")
    print(f"Music: {len(d.music)}")
    print(f"Talk: {len(d.talk)}")
