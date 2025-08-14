use std::path::PathBuf;

fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();

    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    if noargs::VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    noargs::HELP_FLAG.take_help(&mut args);

    let mut input_files = Vec::new();
    while let Some(a) = noargs::arg("[INPUT_FILE]...").take(&mut args).present() {
        input_files.push(PathBuf::from(a.value()));
    }

    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    Ok(())
}
