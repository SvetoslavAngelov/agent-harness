mod domain;

use crate::domain::{message::Role, session::Session};

fn main() {
    let mut session = Session::new();
    session.append_message(Role::User, "Hello World!");

    println!("{session:#?}",);
}
