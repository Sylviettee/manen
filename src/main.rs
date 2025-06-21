use reedline::{DefaultPrompt, Reedline, Signal};

fn main() {
    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt::default();

    loop  {
        let signal = line_editor.read_line(&prompt);

        match signal {
            Ok(Signal::Success(buffer)) => {
                println!("got {buffer}");
            }
            Ok(Signal::CtrlC) | Ok(Signal::CtrlD) => {
                println!("\naborted");
                break
            }
            _ => {}
        }
    }
}
