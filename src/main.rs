use editor::Editor;

mod editor;
mod highlight;
mod format;

fn main() -> color_eyre::Result<()> {
    Editor::new()?.run();
    Ok(())
}
