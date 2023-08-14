use std::path::Path;

use tensorflow::{
    Graph, Operation, SavedModelBundle, SessionOptions, SessionRunArgs, Tensor,
    DEFAULT_SERVING_SIGNATURE_DEF_KEY,
};

pub struct TfModel {
    bundle: SavedModelBundle,
    input: Operation,
    output: Operation,
    output_index: i32,
}

impl TfModel {
    pub fn load<P: AsRef<Path>>(
        dir: P,
        input_name: &'static str,
        output_name: &'static str,
        output_index: i32,
    ) -> anyhow::Result<Self> {
        let mut graph = Graph::new();
        let bundle = SavedModelBundle::load(&SessionOptions::new(), ["serve"], &mut graph, dir)?;

        let signature = bundle
            .meta_graph_def()
            .get_signature(DEFAULT_SERVING_SIGNATURE_DEF_KEY)?;

        let input = {
            let input_info = signature.get_input(input_name)?;
            graph.operation_by_name_required(&input_info.name().name)?
        };

        let output = {
            let output_info = signature.get_output(output_name)?;
            graph.operation_by_name_required(&output_info.name().name)?
        };

        Ok(Self {
            bundle,
            input,
            output,
            output_index,
        })
    }

    pub fn yamnet<P: AsRef<Path>>(dir: P) -> anyhow::Result<Self> {
        Self::load(dir.as_ref().join("yamnet/"), "waveform", "output_1", 1)
    }

    pub fn adbanda<P: AsRef<Path>>(dir: P, name: &str) -> anyhow::Result<Self> {
        Self::load(dir.as_ref().join(name), "embedding", "output", 0)
    }

    pub fn run(&self, input: &Tensor<f32>) -> anyhow::Result<Tensor<f32>> {
        let mut args = SessionRunArgs::new();
        args.add_feed(&self.input, 0, input);

        let token_output = args.request_fetch(&self.output, self.output_index);

        self.bundle.session.run(&mut args)?;
        let output = args.fetch(token_output)?;
        Ok(output)
    }
}
