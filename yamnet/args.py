import argparse
from typing import NoReturn

import configurations


def parse_train() -> configurations.TrainConfig | NoReturn:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "config", choices=["amt", "mo", "at"], help="Model configuration"
    )
    parser.add_argument("dataset_root", type=str, help="Dataset root")
    args = parser.parse_args()

    match args.config:
        case "amt":
            return configurations.AdvertMusicOrTalkConfig(
                model_name="adbanda_amt",
                dataset_root=args.dataset_root,
            )
        case "mo":
            return configurations.MusicOrOtherConfig(
                model_name="adbanda_mo",
                dataset_root=args.dataset_root,
            )
        case "at":
            return configurations.AdvertOrTalkConfig(
                model_name="adbanda_at",
                dataset_root=args.dataset_root,
            )
        case _:
            assert False, "Should never get here"


def parse_predict() -> configurations.PredictionConfig | NoReturn:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "config", choices=["amt", "mo", "at"], help="Model configuration"
    )
    parser.add_argument("input", type=str, help="WAV file for prediction")
    args = parser.parse_args()

    cfg = None
    match args.config:
        case "amt":
            cfg = configurations.AdvertMusicOrTalkConfig(
                model_name="adbanda_amt",
            )
        case "mo":
            cfg = configurations.MusicOrOtherConfig(
                model_name="adbanda_mo",
            )
        case "at":
            cfg = configurations.AdvertOrTalkConfig(
                model_name="adbanda_at",
            )
        case _:
            assert False, "Should never get here"

    return configurations.PredictionConfig(
        model_name=cfg.model_name, class_names=cfg.class_names, input=args.input
    )
