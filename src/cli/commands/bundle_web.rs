use clap::ArgMatches;
use colored::*;
use mech_core::*;

pub(crate) fn run(matches: &ArgMatches) -> MResult<()> {
  let badge = "[Mech Bundle]".truecolor(34, 204, 187);

  let loaded = crate::cli::bundle_web::load_bundle_web_config(matches)?;
  println!("{badge} Loading config… {}", loaded.path.display());

  let options = crate::cli::bundle_web::effective_bundle_web_options(matches, loaded)?;
  let result = crate::bundle_web_project(options)?;

  println!("{badge} Bundle written: {}", result.output_dir.display());
  println!("{badge} Sources bundled: {}", result.source_count);

  Ok(())
}
