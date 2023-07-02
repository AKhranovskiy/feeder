import argparse
from typing import NoReturn

import config


def parse_train() -> config.TrainConfig | NoReturn:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "config", choices=["amt", "mo", "at"], help="Model configuration"
    )
    parser.add_argument("dataset_root", type=str, help="Dataset root")
    args = parser.parse_args()

    match args.config:
        case "amt":
            return config.AdvertMusicOrTalkConfig(
                model_name="adbanda_amt",
                dataset_root=args.dataset_root,
            )
        case "mo":
            return config.MusicOrOtherConfig(
                model_name="adbanda_mo",
                dataset_root=args.dataset_root,
            )
        case "at":
            return config.AdvertOrTalkConfig(
                model_name="adbanda_at",
                dataset_root=args.dataset_root,
            )
        case _:
            assert False, "Should never get here"


def parse_predict() -> config.PredictionConfig | NoReturn:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "config", choices=["amt", "mo", "at"], help="Model configuration"
    )
    parser.add_argument("input", type=str, help="WAV file for prediction")
    args = parser.parse_args()

    cfg = None
    match args.config:
        case "amt":
            cfg = config.AdvertMusicOrTalkConfig(
                model_name="adbanda_amt",
            )
        case "mo":
            cfg = config.MusicOrOtherConfig(
                model_name="adbanda_mo",
            )
        case "at":
            cfg = config.AdvertOrTalkConfig(
                model_name="adbanda_at",
            )
        case _:
            assert False, "Should never get here"

    return config.PredictionConfig(
        model_name=cfg.model_name, class_names=cfg.class_names, input=args.input
    )
