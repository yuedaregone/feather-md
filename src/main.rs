mod app;
mod config;
mod file_association;
mod file_watcher;
mod resources;

use clap::Parser;

/// FeatherMD - 极致轻量的 Markdown 查看器
#[derive(Parser, Debug)]
#[command(name = "FeatherMD", version, about = "极致轻量的 Markdown 查看器")]
struct Args {
    /// 要打开的 Markdown 文件路径
    file: Option<String>,

    /// 注册 .md 文件关联
    #[arg(long)]
    register: bool,

    /// 取消 .md 文件关联
    #[arg(long)]
    unregister: bool,

    /// 指定主题
    #[arg(long)]
    theme: Option<String>,
}

fn main() {
    let args = Args::parse();

    let application = app::App::new();

    // Handle register/unregister
    if args.register {
        match application.register_file_association() {
            Ok(()) => println!("✓ 已注册 .md 文件关联"),
            Err(e) => eprintln!("✗ 注册失败: {}", e),
        }
        return;
    }

    if args.unregister {
        match application.unregister_file_association() {
            Ok(()) => println!("✓ 已取消 .md 文件关联"),
            Err(e) => eprintln!("✗ 取消失败: {}", e),
        }
        return;
    }

    // Apply theme from CLI
    if let Some(theme) = args.theme {
        let mut config = config::Config::load();
        config.update_theme(&theme);
    }

    // Resolve file path
    let file_path = args.file.and_then(|f| {
        match std::fs::canonicalize(&f) {
            Ok(path) => Some(path),
            Err(_) => {
                eprintln!("错误: 文件不存在或路径无效: {}", f);
                None
            }
        }
    });

    application.run(file_path);
}
