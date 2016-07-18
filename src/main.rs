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
            Ok(Response::with((status::NotFound, "Wow. Such 404.")))
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

    // Convert HTML file from request to PDF
    fn convert(req: &mut Request) -> IronResult<Response> {
        let random_name = &format!("{}", Uuid::new_v4());
        let destination_pdf = &{ "./static/".to_string() + random_name + ".pdf" };

        // Get the HTML file
        let html = &match tmp_uploaded_filename(req, "file") {
            Some(tmp_file_name) => {
                let target_file_name = &{ "./static/".to_string() + random_name + ".html" };
                mv_file(&tmp_file_name, target_file_name);
                target_file_name.to_string()
            },
            None => {
                "./templates/fail.html".to_string()
            },
        };

        // Footer
        let footer_html: Option<String> = match tmp_uploaded_filename(req, "footer") {
            Some(tmp_file_name) => {
                let target_file_name = { "./static/".to_string() + random_name + "-footer.html" };
                mv_file(&tmp_file_name, &target_file_name);
                Some(target_file_name)
            },
            _ => {
                None
            },
        };

        // Convert it to HTML
        convert_to_pdf(html, destination_pdf, footer_html);

        Ok(Response::with((status::Ok, format!("http://html2pdf.raph.site/static/{}.pdf\n", random_name).to_string())))
    }
}

fn convert_to_pdf(html: &str, destination_pdf: &str, footer_html: Option<String>) {
    let mut c = Command::new("xvfb-run");

    c.arg("-a")
        .arg("wkhtmltopdf");

    if let Some(f) = footer_html {
        c.arg("--footer-html")
            .arg(&f.to_string());
    }

    c.arg(html)
        .arg(destination_pdf)
        .output()
        .expect("failed to execute process");
}

fn tmp_uploaded_filename(req: &mut Request, param_name: &str) -> Option<String> {
    match req.get_ref::<Params>().unwrap().find(&[param_name]) {
        Some(&Value::File(ref file)) => {
            Some(file.path().to_str().unwrap().to_string())
        },
        _ => {
            None
        },
    }
}

// Move a file (just an mv alias)
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
