extern crate core;

use std::collections::{HashMap, HashSet};
use std::env;
use std::path::Path;

fn main() {
    let mut stack: Vec<&str> = vec![];
    let mut valid_anchors: HashMap<&str, HashSet<&str>> = HashMap::new();

    let grep_project = std::process::Command::new("grep")
        .args(["-I", "-H", "-R", "--exclude-dir=scripts", "ANCHOR", "."])
        .output()
        .expect("failed rg command");

    let output_to_string =
        String::from_utf8(rg_project.stdout).expect("failed to parse command output");

    let split = output_to_string.split('\n');

    let vec_of_anchors = split
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|g| g.replace("./", "").replace(' ', "").replace("//", ""))
        .collect::<Vec<_>>();

    for i in &vec_of_anchors {
        let parts = i.split(':').collect::<Vec<_>>();

        match *parts.get(1).expect("error in parsing vec_of_anchors") {
            "ANCHOR" => {
                stack.push(*parts.get(2).expect("anchor name/tag is missing"));
            }
            "ANCHOR_END" => {
                if stack.is_empty() {
                    panic!(
                        "ANCHOR of \"{}\" is missing or is wrong",
                        *parts.get(2).expect("error")
                    );
                }
                let first_from_stack = stack.pop().unwrap();
                if first_from_stack != *parts.get(2).expect("error") {
                    panic!(
                        "ANCHOR_END of \"{}\" is missing or is wrong",
                        first_from_stack
                    );
                }
            }
            &_ => {
                panic!(
                    "Invalid syntax for ANCHOR or ANCHOR_END:  \"{}\"",
                    *parts.get(1).expect("error")
                );
            }
        }

        if valid_anchors.contains_key(*parts.get(0).unwrap()) {
            let update_hash_set = valid_anchors.get_mut(*parts.get(0).unwrap()).unwrap();
            update_hash_set.insert(*parts.get(2).unwrap());
        } else {
            let mut new_hash_set = HashSet::new();
            new_hash_set.insert(*parts.get(2).unwrap());
            valid_anchors.insert(*parts.get(0).unwrap(), new_hash_set);
        }
    }

    if !stack.is_empty() {
        panic!(
            "ANCHOR_END of \"{}\" is missing or is wrong",
            stack.pop().unwrap()
        );
    }

    let grep_docs = std::process::Command::new("grep")
        .args(["-I", "-H", "-R", "--exclude-dir=scripts", "{{#include", "."])
        .output()
        .expect("failed grep command");

    let output_docs_to_string =
        String::from_utf8(grep_docs.stdout).expect("failed to parse command output");

    let split = output_docs_to_string.split('\n');
    let vec_of_doc_anchors = split
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|g| {
            g.replace(' ', "")
                .replace("{{#include", "")
                .replace("}}", "")
        })
        .collect::<Vec<_>>();

    for i in vec_of_doc_anchors {
        let parts = i.split(':').collect::<Vec<_>>();

        let (_folder_path, _file_name) = (*parts.get(0).unwrap()).rsplit_once('/').unwrap();

        let _ = env::set_current_dir(&_folder_path).is_ok();

        if !Path::new(*parts.get(1).unwrap()).exists() {
            panic!("Cannot find path to \"{}\"", *parts.get(1).expect("error"));
        }

        let doc_file_with_anchors = parts.get(1).unwrap().replace("../", "");

        if parts.get(2).is_some()
            && !valid_anchors
                .get(doc_file_with_anchors.as_str())
                .unwrap()
                .contains(parts.get(2).unwrap())
        {
            panic!(
                "Cannot find anchor \"{}\" on path \"{}\"",
                *parts.get(2).unwrap(),
                *parts.get(1).unwrap()
            );
        }
    }
}
