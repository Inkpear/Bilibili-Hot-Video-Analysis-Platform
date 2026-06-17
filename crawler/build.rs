fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(&["src/protos/danmaku.proto"], &["src/protos/"])?;
    Ok(())
}
