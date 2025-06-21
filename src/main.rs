use editor::Editor;

mod editor;
mod format;
mod highlight;

fn main() -> color_eyre::Result<()> {
    Editor::new()?.run();
    Ok(())
}
