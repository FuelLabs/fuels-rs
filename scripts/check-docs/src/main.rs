extern crate core;

use std::collections::HashSet;
use std::env;
use std::path::Path;
use std::process::Stdio;

fn main() {
    let mut stack = vec![];
    let mut valid_anchors = HashSet::new();

    let rg_project = std::process::Command::new("rg")
    .arg("--case-sensitive")
    .arg("-g")
    .arg("!/scripts/check-docs/src/main.rs")
    .arg("ANCHOR")
    .stdout(Stdio::piped())
    .spawn()
    .expect("failed rg command");

    let output_files = std::process::Command::new("sed")
    .arg("s/ //g; s=//==g")
    .stdin(rg_project.stdout.unwrap())
    .output()
    .expect("failed sed command");


    let output_to_string =
        String::from_utf8(output_files.stdout).expect("failed to parse command output");
    let split = output_to_string.split('\n');
    let vec_of_anchors = split
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>();

    for i in vec_of_anchors {
        let parts = i.split(':').collect::<Vec<&str>>();
        if *parts.get(1).expect("error in parsing vec_of_anchors") == "ANCHOR" {
            stack.push(*parts.get(2).expect("error"));
            // println!("{:?}", stack);
        }
        else if *parts.get(1).expect("error in parsing vec_of_anchors") == "ANCHOR_END" {
            // TODO test this
            if stack.is_empty() {
                panic!(
                    "ANCHOR of \"{}\" cannot be found",
                    *parts.get(2).expect("error")
                );
            }

            if stack.pop().unwrap() != *parts.get(2).expect("error") {
                panic!(
                    "ANCHOR or ANCHOR_END of \"{}\" is wrong",
                    *parts.get(2).expect("error")
                );
            }
            // println!("{:?}", stack);
        } else {
            panic!(
                "Invalid syntax for ANCHOR or ANCHOR_END\"{}\"",
                *parts.get(1).expect("error")
            );
        }

        valid_anchors.insert(*parts.get(2).expect("error in parsing vec_of_anchors"));

    }

    let rg_docs = std::process::Command::new("rg")
        .arg("--case-sensitive")
        .arg("-g")
        .arg("!/scripts/check-docs/src/main.rs")
        .arg("\\{\\{#include")
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed rg command");

    let output_docs = std::process::Command::new("sed")
        .arg("s/ //g; s={{#include==g; s=}}==g")
        .stdin(rg_docs.stdout.unwrap())
        .output()
        .expect("failed sed command");

    let output_docs_to_string = String::from_utf8(output_docs.stdout).expect("failed to parse command output");

    let split = output_docs_to_string.split('\n');
    let vec_of_doc_anchors = split
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>();

    for i in vec_of_doc_anchors {
        let parts = i.split(':').collect::<Vec<&str>>();

        let mut root = (*parts.get(0).unwrap()).rsplitn(2,"/");
        let _file_name = root.next().unwrap();
        let _folder_path = root.next().unwrap();

        let _ = env::set_current_dir(&_folder_path).is_ok();

        if !Path::new(*parts.get(1).unwrap()).exists() {
           panic!("Cannot find path to \"{}\"", *parts.get(1).expect("error"));
        }

        if parts.get(2).is_some() && !valid_anchors.contains(*parts.get(2).unwrap()){
            panic!("Cannot find anchor \"{}\" on path \"{}\"", *parts.get(2).unwrap(), *parts.get(1).unwrap());
        }


    }

}
