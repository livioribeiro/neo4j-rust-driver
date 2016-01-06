extern crate neo4j_rust_driver as neo4j;
#[macro_use]
extern crate log;
extern crate env_logger;

fn main() {
    env_logger::init();
    let mut conn = neo4j::connect("localhost", 7687).unwrap();
    let init = conn.init("rust-driver/0.1");

    println!("{:?}", init);
}
