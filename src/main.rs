mod app;
mod editor;
mod workspace;

fn main() -> anyhow::Result<()> {
    app::run()
}
