use std::path::PathBuf;

/// 定位可用的 protoc（protoc compiler）：
/// 1. 显式 `PROTOC` 环境变量指向的文件；
/// 2. PATH 中的系统 protoc（如 250 runner 镜像已预装）；
/// 3. 两者都不可用时回退到仓库内置的 vendored protoc。
///
/// 背景：GitHub 托管 runner（ubuntu-latest / ubuntu-26.04）的官方镜像清单中
/// 没有 protobuf-compiler，`cargo build --workspace` 触发 crates/network 的
/// build.rs 时会因找不到 protoc 直接失败。此处回退到 crates.io 分发的
/// vendored 二进制，保证任何 runner 都能完成 workspace 构建。
fn resolve_protoc() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("PROTOC") {
        let path = PathBuf::from(explicit);
        if path.is_file() {
            return Some(path);
        }
    }
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("protoc");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=PROTOC");
    println!("cargo:rerun-if-env-changed=PATH");
    // 无论走哪条分支，都把最终选定的 protoc 显式写回 PROTOC，
    // 避免环境里存在失效的 PROTOC 值时 tonic 仍然沿用它。
    match resolve_protoc() {
        Some(found) => {
            std::env::set_var("PROTOC", &found);
        }
        None => {
            let vendored = protoc_bin_vendored::protoc_bin_path()?;
            std::env::set_var("PROTOC", &vendored);
            println!(
                "cargo:warning=系统 protoc 缺失，回退到 vendored protoc: {}",
                vendored.display()
            );
        }
    }
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["protos/dtc.proto"], &["protos/"])?;
    Ok(())
}
