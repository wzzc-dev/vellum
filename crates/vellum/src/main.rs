mod app;
mod logging;
mod path;

fn main() -> anyhow::Result<()> {
    logging::init();
    app::run()
}
