import os
from multiprocessing import Pool

import tensorflow as tf

import args
import util

dataset_root = args.parse_embeddings()

print(f"Prepare embeddings for {dataset_root}")

print("Loading YAMNET model")
yamnet_model = tf.saved_model.load("models/yamnet")


def process(name):
    ds = tf.data.Dataset.list_files(
        f"{dataset_root}/{name}/*.wav",
        shuffle=False,
        name=name,
    )
    print(f"{name}: {len(ds)} files")

    def filepath_to_embeddings(filename):
        audio_wav = util.load_16k_audio_wav(filename)
        _, embeddings, _ = yamnet_model(audio_wav)  # type: ignore
        return embeddings

    ds = ds.map(
        lambda x: filepath_to_embeddings(x),
        num_parallel_calls=tf.data.AUTOTUNE,
    ).unbatch()  # type: ignore

    path = os.path.join(dataset_root, f"{name}.embeddings")
    print(f"saving {path}")
    ds.save(path, compression="GZIP")
    ds = tf.data.Dataset.load(path, compression="GZIP")
    print(f"{name}: {len(ds)} embeddings")


with Pool(3) as p:
    p.map(process, ["advert", "music", "talk"])
