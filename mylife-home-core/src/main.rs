use std::alloc::System;

#[global_allocator]
static ALLOCATOR: System = System;

fn main() {
    println!("Hello, world!");
}
