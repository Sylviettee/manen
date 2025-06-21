use editor::Editor;
mod editor;

fn main() -> color_eyre::Result<()> {
    Editor::new()?.run();
    Ok(())
}
