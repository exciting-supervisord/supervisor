mod terminal;

use terminal::Terminal;
fn main() {
    let mut t = Terminal::new("prompt>");

    loop {
        println!("\n{}", t.getline());
    }
}
