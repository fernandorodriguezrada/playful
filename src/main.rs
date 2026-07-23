use color_eyre::eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    let config = playful::config::Config::load().unwrap_or_default();
    let mut app = playful::app::App::new(config);
    app.run()?;

    Ok(())
}
