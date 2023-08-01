import argparse
from typing import NoReturn

import configurations


def parse_train() -> configurations.TrainConfig | NoReturn:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "config", choices=["amt", "mo", "at", "ao"], help="Model configuration"
    )
    parser.add_argument("dataset_root", type=str, help="Dataset root")
    args = parser.parse_args()

    match args.config:
        case "amt":
            return configurations.TrainConfig(
                configurations.AMT_MODEL,
                data_dir=args.dataset_root,
            )
        case "mo":
            return configurations.TrainConfig(
                configurations.MO_MODEL,
                data_dir=args.dataset_root,
            )
        case "at":
            return configurations.TrainConfig(
                configurations.AT_MODEL,
                data_dir=args.dataset_root,
            )
        case "ao":
            return configurations.TrainConfig(
                configurations.AO_MODEL,
                data_dir=args.dataset_root,
            )
        case _:
            assert False, "Should never get here"


def parse_predict() -> configurations.PredictionConfig | NoReturn:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "config", choices=["amt", "mo", "at", "ao"], help="Model configuration"
    )
    parser.add_argument("input", type=str, help="WAV file for prediction")
    args = parser.parse_args()

    match args.config:
        case "amt":
            return configurations.PredictionConfig(
                model_config=configurations.AMT_MODEL, input=args.input
            )
        case "mo":
            return configurations.PredictionConfig(
                model_config=configurations.MO_MODEL, input=args.input
            )
        case "at":
            return configurations.PredictionConfig(
                model_config=configurations.AT_MODEL, input=args.input
            )
        case "at":
            return configurations.PredictionConfig(
                model_config=configurations.AO_MODEL, input=args.input
            )
        case _:
            assert False, "Should never get here"


def parse_embeddings() -> str:
    parser = argparse.ArgumentParser()
    parser.add_argument("dataset_root", type=str, help="Dataset root")
    return parser.parse_args().dataset_root
