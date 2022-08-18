use std::process::Stdio;

fn main() {

    let rg = std::process::Command::new("rg")
        .arg("--case-sensitive")
        .arg("-g")
        .arg("!/scripts/check-docs/src/main.rs")
        .arg("ANCHOR")
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed rg command");

    let output = std::process::Command::new("sed")
        .arg("s/ //g; s=//==g")
        .stdin(rg.stdout.unwrap()) // Converted into a Stdio here
        .output()
        .expect("failed sed command");

    println!("\n \n");

    let output_to_string = String::from_utf8(output.stdout).expect("failed to parse command output");

    let mut split = output_to_string.split("\n");
    let vec = split.collect::<Vec<&str>>();


}
