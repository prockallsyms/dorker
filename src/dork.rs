extern crate reqwest;
extern crate select;

use core::convert::AsMut;
use futures::executor::block_on;
use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;
use select::document::Document;
use select::node::Node;
use select::predicate::{Attr, Class, Element, Name, Predicate, Text};
use std::collections::HashMap;
use std::error::Error;
use std::str;

struct BoxedPred(Box<dyn Predicate>);

impl Predicate for BoxedPred {
    fn matches(&self, node: &Node) -> bool {
        self.0.matches(node)
    }
}

pub async fn get_body(url: &str) -> String {
    Client::new()
        .get(url)
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.13; rv:72.0) Gecko/20100101 Firefox/72.0",
            //"Mozilla/5.0 (Macintosh; Intel Mac OS X 10.10; rv:34.0) Gecko/20100101 Firefox/34.0",
        )
        .send()
        .unwrap()
        .text()
        .unwrap()
}

pub fn find_links(
    engine: &str,
    dorks: HashMap<String, String>,
    extra: Option<String>,
) -> Result<Vec<QueryItem>, Box<dyn Error>> {
    let mut request_string = format!("https://{}/search?q=", engine);

    for (dork, content) in dorks {
        request_string.push_str(dork.as_str());
        request_string.push(':');
        request_string.push_str(content.as_str());
        request_string.push_str("%20");
    }

    if let Some(x) = extra {
        request_string.push_str(&x);
    }

    // If the engine is not found, we just use google's predicate
    let pred: BoxedPred = match engine {
        "google.com" => BoxedPred(Box::new(
            Attr("id", "rso")
                .descendant(Attr("class", "bkWMgd"))
                .descendant(Attr("class", "r"))
                .descendant(Name("a")),
        )),
        // I Think this one works, needs more testing
        "www.bing.com" => BoxedPred(Box::new(
            Attr("id", "b_content")
                .descendant(Attr("id", "b_results"))
                .descendant(Attr("class", "b_algo"))
                .descendant(Name("a")),
        )),
        // These currently don't work
        /* "duckduckgo.com" => BoxedPred(Box::new(
            Attr("id", "links_wrapper")
                .descendant(Attr("class", "results"))
                .descendant(Attr("class", "result__a"))
                .descendant(Name("a")),
        )),
        "www.ecosia.org" => BoxedPred(Box::new(
            Attr("class", "mainline-results")
                .descendant(Attr("class", "result"))
                .descendant(Attr("class", "result-url"))
                .descendant(Name("a")),
        )), */
        _ => BoxedPred(Box::new(
            Attr("id", "rso")
                .descendant(Attr("class", "bkWMgd"))
                .descendant(Attr("class", "r"))
                .descendant(Name("a")),
        )),
    };

    let body = block_on(get_body(request_string.as_str()));
    let document = Document::from(body.as_str());
    let mut link_items: Vec<QueryItem> = Vec::new();
    for node in document.find(pred) {
        let title_pred: BoxedPred = match engine {
            "google.com" => BoxedPred(Box::new(Class("LC20lb"))),
            "www.bing.com" => BoxedPred(Box::new(Text)),
            // These don't work right now
            /* "duckduckgo.com" => BoxedPred(Box::new(Element)),
            "www.ecosia.org" => BoxedPred(Box::new(Element)), */
            _ => BoxedPred(Box::new(Class("LC20lb"))),
        };

        let link = node.attr("href").unwrap();
        for title in node.find(title_pred) {
            link_items.push(QueryItem::new(title.text(), link.to_string()))
            /* println!(
                "Debug: {}: found title {} for link {}",
                engine,
                title.text(),
                link.to_string()
            ); */
        }
    }

    Ok(link_items)
}

#[derive(Debug)]
pub struct DorkResults {
    accumulated: Vec<DorkResult>,
}

impl DorkResults {
    pub fn new() -> DorkResults {
        let res = DorkResults {
            accumulated: vec![],
        };
        res
    }

    pub fn add(&mut self, result: DorkResult) -> Result<(), Box<dyn Error>> {
        self.accumulated.push(result);
        Ok(())
    }
}

#[derive(Debug)]
pub struct DorkResult {
    engine: String,
    urls: Vec<String>,
}

impl DorkResult {
    pub fn new() -> DorkResult {
        DorkResult {
            engine: String::from(""),
            urls: vec![],
        }
    }

    pub fn set_engine(&mut self, engine: String) {
        self.engine = engine;
    }

    pub fn add_url(&mut self, url: String) {
        self.urls.push(url)
    }
}

#[derive(Debug)]
pub struct Dork {
    engine: String,
    dorks: HashMap<String, String>,
    extra: Option<String>,
}

impl Dork {
    pub fn new() -> Dork {
        Dork {
            engine: String::from(""),
            dorks: HashMap::new(),
            extra: None,
        }
    }

    pub fn from(engine: String, dorks: HashMap<String, String>, extra: String) -> Dork {
        Dork {
            engine,
            dorks,
            extra: Some(extra.to_string()),
        }
    }

    pub fn get_scrape(&self) {
        find_links(&self.engine, self.dorks.clone(), self.extra.clone());
    }
}

#[derive(Debug, Clone)]
pub struct QueryItem {
    pub link: String,
    pub title: String,
}

impl QueryItem {
    pub fn new(title: String, link: String) -> QueryItem {
        QueryItem { title, link }
    }
}
