cargo run --release --bin train -- ../artifacts/
cp -r model/ ../artifacts/model_(date +%F-%H%M)/
