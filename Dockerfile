# Build restreamer
FROM rust:latest as rust-builder

WORKDIR /usr/src/app

COPY . .
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    libavutil-dev libavcodec-dev libavformat-dev libswscale-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install --path restreamer

# final stage
FROM debian:11 as final

RUN apt-get update \
    && apt-get install --no-install-recommends -y ffmpeg \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=rust-builder /usr/local/cargo/bin/restreamer /usr/local/bin/restreamer

# TODO There should a better way to obtain Tensorflow libraries
COPY --from=rust-builder /usr/src/app/target/release/build/tensorflow-sys-*/out/libtensorflow.so.2 /usr/local/lib/
COPY --from=rust-builder /usr/src/app/target/release/build/tensorflow-sys-*/out/libtensorflow_framework.so.2 /usr/local/lib/

RUN ldconfig

COPY yamnet/models/ models/
COPY restreamer/assets restreamer/assets

ENV TF_CPP_MIN_LOG_LEVEL=3

CMD restreamer --port 8192 --gcp

EXPOSE 8192
