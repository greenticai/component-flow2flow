fn main() {
    match flow2flow_cli::run_from_env() {
        Ok(output) => {
            println!("{output}");
        }
        Err(err) => {
            eprintln!("flow2flow-cli error: {err}");
            std::process::exit(1);
        }
    }
}
