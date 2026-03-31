mod app;
mod editor;
mod path;
mod workspace;

fn main() -> anyhow::Result<()> {
    app::run()
}
