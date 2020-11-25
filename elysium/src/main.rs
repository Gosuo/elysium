use elysium_core::Elysium;

fn main() {
    env_logger::init();
    let elysium = Elysium::new();
    elysium.run();
    println!("Hello from main");
}
