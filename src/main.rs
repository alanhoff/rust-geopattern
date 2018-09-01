extern crate geopattern;
extern crate iron;
extern crate router;

use iron::headers::{CacheControl, CacheDirective, ContentType};
use iron::mime::{Mime, SubLevel, TopLevel};
use iron::prelude::*;
use router::Router;

fn generate(req: &mut Request) -> IronResult<Response> {
    let hash = req
        .extensions
        .get::<Router>()
        .unwrap()
        .find("hash")
        .unwrap();
    let pattern = geopattern::generate(hash);
    let mut res = Response::with((iron::status::Ok, pattern.to_svg().unwrap().to_string()));

    res.headers.set(ContentType(Mime(
        TopLevel::Image,
        SubLevel::Ext("svg+xml".to_string()),
        vec![],
    )));

    res.headers.set(CacheControl(vec![
        CacheDirective::Public,
        CacheDirective::MaxAge(2592000),
    ]));

    Ok(res)
}

fn main() {
    let mut router = Router::new();
    router.get("/generate/:hash", generate, "generate");

    Iron::new(router).http("0.0.0.0:3000").unwrap();
}
