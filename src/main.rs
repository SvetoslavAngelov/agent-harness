//! Demonstrates the harness's domain model with a single in-memory session.

mod client;
mod domain;
mod session;

use crate::domain::{message::Role, session::Session};

fn main() {
    let mut session = Session::new();
    session.append_text(Role::User, "Hello World!");

    println!("{session:#?}");
}
