use std::fs;
use std::path::PathBuf;
use std::convert::TryFrom;

const SHADERS: &[&str] = &[
    "./shaders/quad.vert",
    "./shaders/quad.frag"
];

struct ShaderData {
    src: String,
    src_path: PathBuf,
    spv_path: PathBuf,
    kind: shaderc::ShaderKind
}

impl ShaderData {
    fn load(src_path: PathBuf) -> Self {
        let extension = src_path.extension()
            .expect("File has no extension")
            .to_str()
            .expect("Extension cannot be converted to &str");

        let kind = match extension {
            "vert" => shaderc::ShaderKind::Vertex,
            "frag" => shaderc::ShaderKind::Fragment,
            "comp" => shaderc::ShaderKind::Compute,
            _ => panic!("Unsupported shader: {}", src_path.display()),
        };

        let src = fs::read_to_string(src_path.clone()).expect("fail to read shader file");
        let spv_path = src_path.with_extension(format!("{}.spv", extension));

        Self { src, src_path, spv_path, kind }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=shaders/*");

    let mut compiler = shaderc::Compiler::new().unwrap();

    for src_path in SHADERS {
        let src_path = PathBuf::try_from(src_path).unwrap();

        let shader = ShaderData::load(src_path);

        let compiled = compiler.compile_into_spirv(
            &shader.src,
            shader.kind,
            &shader.src_path.to_str().unwrap(),
            "main",
            None
        ).unwrap();

        fs::write(shader.spv_path, compiled.as_binary_u8())
            .expect("failed to write SPIR-V file");
    }
}
