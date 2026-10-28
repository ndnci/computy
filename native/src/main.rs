use clap::Parser;
fn main() {
    match computy_engine::cli::run(computy_engine::cli::Cli::parse()) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("Computy: {error}");
            std::process::exit(1);
        }
    }
}
