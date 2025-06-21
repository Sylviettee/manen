use editor::Editor;

mod editor;
mod format;
mod highlight;
mod validator;

fn main() -> color_eyre::Result<()> {
    Editor::new()?.run();
    Ok(())
}
