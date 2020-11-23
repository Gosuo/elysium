use elysium_core::Elysium;

fn main() {
    let elysium = Elysium::new();
    //elysium.init();
    let end = elysium.run();
    println!("Hello from main");
    end.join().expect("Couldn't join the render thread");
}
