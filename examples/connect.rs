// extern crate neo4j_rust_driver as neo4j;
// #[macro_use]
// extern crate log;
// extern crate env_logger;
//
// fn main() {
//     env_logger::init().unwrap();
//     let mut conn = neo4j::connect("localhost", 7687).unwrap();
//     let init = conn.init("MyClient/1.0");
//
//     println!("{:?}", init);
//
//     let result = conn.run("return 1 as one, 2 as two, 3 as three");
//
//     println!("{:?}", result);
//
//     let result = conn.pull_all();
//
//     println!("{:?}", result);
// }

fn main() {}
