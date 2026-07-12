use std::process::ExitCode;

fn main() -> ExitCode {
  match mech::cli::app::run() {
    Ok(()) => ExitCode::SUCCESS,
    Err(error) => {
      if let Err(diagnostic_error) = mech::cli::diagnostics::print_mech_error(&error) {
        eprintln!("failed to print diagnostic: {diagnostic_error}");
      }
      ExitCode::FAILURE
    }
  }
}
