extern crate iron;
extern crate router;
extern crate handlebars_iron;
extern crate params;
extern crate uuid;

use iron::prelude::*;
use iron::{AfterMiddleware};
use iron::status;
use router::{Router, NoRoute};
use handlebars_iron::{Template, HandlebarsEngine, DirectorySource};
use std::collections::BTreeMap;
use params::Params;
use params::Value;
use std::process::Command;
use uuid::Uuid;

struct Custom404;

impl AfterMiddleware for Custom404 {
    fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
        println!("Hitting custom 404 middleware");
        if let Some(_) = err.error.downcast::<NoRoute>() {
            Ok(Response::with((status::NotFound, "Custom 404 response")))
        } else {
            println!("{:?}", err);
            Err(err)
        }
    }
}

fn main() {
    // Templates
    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./templates", ".hbs")));
    if let Err(_) = hbse.reload() {
        panic!("Unable to build templates");
    }

    // Routes
    let mut router = Router::new();
    router.get("/", welcome);
    router.post("/", convert);

    // Full chain
    let mut chain = Chain::new(router);
    chain.link_after(hbse);
    chain.link_after(Custom404);
    Iron::new(chain).http("localhost:3000").unwrap();

    // Welcome route
    fn welcome(_: &mut Request) -> IronResult<Response> {
        let mut resp = Response::new();
        let mut data = BTreeMap::new();

        data.insert("version".to_string(), "1.0".to_string());
        resp.set_mut(Template::new("welcome", data)).set_mut(status::Ok);
        Ok(resp)
    }

    // Convert HTML to PDF
    fn convert(req: &mut Request) -> IronResult<Response> {
        let random_name = &format!("{}", Uuid::new_v4());
        let tmp_file_name = &tmp_uploaded_filename(req, "file");
        let destination_html = &{"./static/".to_string() + random_name + ".html"};
        let destination_pdf = &{"./static/".to_string() + random_name + ".pdf"};

        mv_file(tmp_file_name, destination_html);
        convert_to_pdf(destination_html, destination_pdf);

        Ok( Response::with((status::Ok, format!("http://html2pdf.raph.site/static/{}.pdf", random_name).to_string() )) )
    }
}

fn convert_to_pdf(destination_html: &str, destination_pdf: &str,) {
    Command::new("xvfb-run")
        .arg("wkhtmltopdf")
        .arg(destination_html)
        .arg(destination_pdf)
        .spawn()
        .expect("failed to execute process");
}

fn tmp_uploaded_filename(req: &mut Request, param_name: &str) -> String {
    match req.get_ref::<Params>().unwrap().find(&[param_name]) {
        Some(&Value::File(ref file)) => {
            file.path().to_str().unwrap().to_string()
        },
        _ => {
            panic!("no file") // to do default file with error text
        },
    }
}

fn mv_file(source_name: &str, destination_name: &str) {
    let output = Command::new("mv")
        .arg(source_name)
        .arg(destination_name)
        .output()
        .expect("failed to execute process");
    println!(
        "status: {} stderr: {} stdout: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
}
