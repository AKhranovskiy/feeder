cargo run --release --bin train -- ../artifacts/
cp -r model/ ../artifacts/model_$(date +%Y%m%d-%H%M)/
