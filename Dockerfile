# Build restreamer
FROM rust:latest as rust-builder

WORKDIR /usr/src/app

COPY . .
RUN apt-get update \
    && apt-get install -y python3-dev libavutil-dev libavcodec-dev libavformat-dev libswscale-dev libaubio-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install --path restreamer

# Prepare Python virtual env
FROM python:3.9-slim as python-builder

ENV PYTHONDONTWRITEBYTECODE 1
ENV PYTHONUNBUFFERED 1

WORKDIR /app

RUN python -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

COPY requirements.txt .
RUN pip install -r requirements.txt

# final stage
FROM python:3.9-slim as final

RUN apt-get update && apt-get install -y ffmpeg libaubio5 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=rust-builder /usr/local/cargo/bin/restreamer /usr/local/bin/restreamer
COPY --from=python-builder /opt/venv /opt/venv

COPY tools/model model
RUN mkdir recordings

ENV PATH="/opt/venv/bin:$PATH"

CMD restreamer --port 8192 --gcp

EXPOSE 8192

