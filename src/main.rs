extern crate geopattern;
extern crate iron;
extern crate resvg;
extern crate router;

use geopattern::patterns::Patterns;
use iron::headers::{CacheControl, CacheDirective, ContentType};
use iron::mime::{Mime, SubLevel, TopLevel};
use iron::prelude::*;
use resvg::backend_cairo::render_to_image;
use resvg::usvg;
use resvg::SizeExt;
use router::Router;

const LUMINANCE: f64 = 80.0;

#[derive(Debug)]
enum Mode {
    Unknown,
    Png,
    Svg,
}

#[derive(Debug)]
struct Pattern {
    pub hash: String,
    pub size: u32,
    pub mode: Mode,
}

impl Pattern {
    pub fn from_request(req: &Request) -> Self {
        let hash = req
            .extensions
            .get::<Router>()
            .unwrap()
            .find("hash")
            .unwrap_or("default")
            .into();

        let size = req
            .extensions
            .get::<Router>()
            .unwrap()
            .find("size")
            .unwrap_or("128")
            .parse::<u32>()
            .unwrap_or(128);

        let mode = match req
            .extensions
            .get::<Router>()
            .unwrap()
            .find("mode")
            .unwrap_or("unknown")
            .into()
        {
            "png" => Mode::Png,
            "svg" => Mode::Svg,
            _ => Mode::Unknown,
        };

        Pattern { hash, size, mode }
    }

    pub fn to_svg(&self) -> String {
        geopattern::GeoPattern::new(&self.hash)
            .patterns(&[
                Patterns::Plaid,
                Patterns::ConcentricCircles,
                Patterns::MosaicSquares,
                Patterns::Xes,
                Patterns::Octagons,
                Patterns::OverlappingCircles,
                Patterns::PlusSigns,
                Patterns::Squares,
            ])
            .build()
            .unwrap()
            .to_minified_svg()
            .unwrap()
    }

    pub fn rasterize(&self) -> Vec<u8> {
        let svg = self.to_svg();
        let mut buffer = vec![];

        let mut opts = resvg::Options::default();
        opts.usvg.dpi = 300.0;

        let rtree = usvg::Tree::from_str(&svg, &opts.usvg).unwrap();
        let screensize = rtree.svg_node().size.to_screen_size();

        let zoom = self.size as f32 / std::cmp::min(screensize.width, screensize.height) as f32;
        opts.fit_to = resvg::FitTo::Zoom(zoom);

        let mut surface = render_to_image(&rtree, &opts).unwrap();

        loop {
            let mut data = surface.get_data().unwrap();
            let mut total_luminance: f64 = 0.0;
            let mut pixels: usize = 0;

            for i in (0..data.len()).step_by(4) {
                let r = *data.get(i).unwrap() as f64;
                let g = *data.get(i + 1).unwrap() as f64;
                let b = *data.get(i + 2).unwrap() as f64;

                pixels += 1;
                total_luminance += 0.299 * r + 0.587 * g + 0.114 * b;
            }

            let mean_luminance = total_luminance / pixels as f64;

            if mean_luminance < LUMINANCE {
                for i in (0..data.len()).step_by(4) {
                    for a in 0..3 {
                        let color = data.get_mut(i + a).unwrap();

                        if *color as f64 * 1.1 <= 255.0 {
                            *color = (*color as f64 * 1.1) as u8;
                        }
                    }
                }
            } else {
                break;
            }
        }

        surface.write_to_png(&mut buffer).unwrap();

        buffer
    }

    pub fn build_response(&self) -> IronResult<Response> {
        match self.mode {
            Mode::Unknown => Ok(Response::with((iron::status::NotFound, "Not found"))),
            Mode::Svg => {
                let svg = self.to_svg();
                let mut res = Response::with((iron::status::Ok, svg));

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
            Mode::Png => {
                let buffer = self.rasterize();
                let mut res = Response::with((iron::status::Ok, buffer));

                res.headers.set(ContentType::png());

                res.headers.set(CacheControl(vec![
                    CacheDirective::Public,
                    CacheDirective::MaxAge(2592000),
                ]));

                Ok(res)
            }
        }
    }
}

fn generate(req: &mut Request) -> IronResult<Response> {
    Pattern::from_request(&req).build_response()
}

fn main() {
    resvg::init();

    let mut router = Router::new();
    router.get("/:mode/:hash", generate, "generate");
    router.get("/:mode/:hash/:size", generate, "generate_size");

    Iron::new(router).http("0.0.0.0:3000").unwrap();
}
