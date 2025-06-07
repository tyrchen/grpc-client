use std::process::Command;

fn main() {
    let mut config = tonic_build::Config::new();
    config.format(true);
    tonic_build::configure()
        .out_dir("src/pb")
        .file_descriptor_set_path("src/pb/example.bin")
        .compile_protos_with_config(
            config,
            &["../fixtures/protos/example.proto"],
            &["../fixtures/protos"],
        )
        .unwrap();

    // run `cargo fmt`
    Command::new("cargo").arg("fmt").output().unwrap();
}
