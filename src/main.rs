use color_eyre::eyre::Result;

const HELP: &str = concat!(
    "♪ Playful - Terminal Music Player\n",
    "Usage: playful [--help]\n\n",
    "Keybindings:\n",
    "  ↑↓/jk       Navigate\n",
    "  Enter       Play selected\n",
    "  Space       Play/Pause\n",
    "  n/p         Next/Prev\n",
    "  s           Stop\n",
    "  +/-         Volume\n",
    "  .,          Seek ±5s\n",
    "  c           Change folder\n",
    "  r           Refresh\n",
    "  q           Quit\n",
    "  Alt+;       Command palette\n",
);

fn main() -> Result<()> {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        print!("{HELP}");
        return Ok(());
    }

    color_eyre::install()?;

    let config = playful::config::Config::load().unwrap_or_default();
    let mut app = playful::app::App::new(config);
    app.run()?;

    Ok(())
}
