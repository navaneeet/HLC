use hlc::cli;

fn main() {
	env_logger::init();
	if let Err(e) = cli::run() {
		eprintln!("Error: {}", e);
		std::process::exit(1);
	}
}