use std::process::Stdio;

fn main() {
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

    let mut stack = vec![];

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
        } else {
            if stack.is_empty() {
                panic!(
                    "ANCHOR of \"{}\" cannot be found",
                    *parts.get(2).expect("error")
                );
            }

            if stack.pop().unwrap() != *parts.get(2).expect("error") {
                panic!(
                    "ANCHOR_END of \"{}\" cannot be found",
                    *parts.get(2).expect("error")
                );
            }
            // println!("{:?}", stack);
        }
    }

    // println!("vani : {:?}", stack);

    let _rg_docs = std::process::Command::new("rg")
        .arg("--case-sensitive")
        .arg("--no-filename")
        .arg("\\{\\{#include")
        .arg("fuels-rs/docs")
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed rg command");
    //
    // let output_docs = std::process::Command::new("sed")
    //     .arg("s={{#include==g; s=}}==g")
    //     .stdin(rg_project.stdout.unwrap())
    //     .output()
    //     .expect("failed sed command");

    // let output_docs_to_string = String::from_utf8(output_docs.stdout).expect("failed to parse command output");
    //
    // println!("{:?}", output_docs)
}
