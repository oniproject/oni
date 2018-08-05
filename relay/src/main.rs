#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::{
    Rocket,
    response::{
        NamedFile,
        Redirect,
        status::NotFound,
    },
};

use std::path::{Path, PathBuf};

#[get("/")]
fn index() -> Redirect {
    Redirect::to("/index.html")
}

#[get("/<file..>")]
fn files(file: PathBuf) -> Result<NamedFile, NotFound<String>> {
    let path = Path::new("static/").join(file);
    NamedFile::open(&path).map_err(|_| NotFound(format!("Bad path: {:?}", path)))
}

fn rocket() -> Rocket {
    rocket::ignite()
        .mount("/", routes![
            index,
            files,
        ])
}

fn main() {
    rocket().launch();
}
