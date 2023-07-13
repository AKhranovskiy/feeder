# Build restreamer
FROM rust:slim-bookworm as rust-builder

WORKDIR /usr/src/app

COPY . .
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    python3-dev libavutil-dev libavcodec-dev libavformat-dev libswscale-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install --path restreamer

# Prepare Python virtual env
FROM python:3.11-slim-bookworm as python-builder

ENV PYTHONDONTWRITEBYTECODE 1
ENV PYTHONUNBUFFERED 1

WORKDIR /app

RUN python -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

COPY requirements.docker.txt requirements.txt
RUN pip install --upgrade pip
RUN pip install -r requirements.txt

# final stage
FROM python:3.11-slim-bookworm as final

RUN apt-get update \
    && apt-get install --no-install-recommends -y ffmpeg \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=rust-builder /usr/local/cargo/bin/restreamer /usr/local/bin/restreamer
COPY --from=python-builder /opt/venv /opt/venv

COPY models/ models/
COPY restreamer/assets restreamer/assets

ENV PATH="/opt/venv/bin:$PATH"
ENV TF_CPP_MIN_LOG_LEVEL=3

CMD restreamer --port 8192 --gcp

EXPOSE 8192
