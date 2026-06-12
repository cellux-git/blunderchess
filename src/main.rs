use blunderchess::uci::Engine;

fn main() {
    env_logger::init();
    blunderchess::attack::init_slider_tables();
    log::info!("BlunderChess starting");

    let mut engine = Engine::new();
    let stdin = std::io::stdin();
    let mut buf = String::new();

    loop {
        buf.clear();
        match stdin.read_line(&mut buf) {
            Ok(0) => break,
            Ok(_) => {
                if !engine.process_command(&buf) {
                    break;
                }
            }
            Err(e) => {
                log::error!("Error reading stdin: {e}");
                break;
            }
        }
    }

    log::info!("BlunderChess exiting");
}
