extern crate skim;
use i3ipc::reply::Node;
use i3ipc::I3Connection;
use itertools::Itertools;
use skim::prelude::*;
use std::env;
use std::io::Cursor;

fn main() {
    let mut ipc = I3Connection::connect().expect("failed to connect");
    let tree = ipc.get_tree().unwrap();
    let i3node: &Node = tree.nodes.get(1).unwrap();
    let workspaces: &Vec<Node> = &i3node.nodes;

    // "consume" the values returned by the functions and digest them into a new String.
    // See https://stackoverflow.com/questions/40792801/best-way-to-concatenate-vectors-in-rust
    let all_choices: String = running_programs(workspaces)
        .into_iter()
        .chain(launchable_programs(path()).into_iter())
        .map(|program| format!("{}\n", program))
        .collect();

    // `SkimItemReader` is a helper to turn any `BufRead` into a stream of `SkimItem`
    // `SkimItem` was implemented for `AsRef<str>` by default
    let items = SkimItemReader::default().of_bufread(Cursor::new(all_choices));
    let options = SkimOptionsBuilder::default().multi(false).build().unwrap();

    // `run_with` would read and show items from the stream
    let item = Skim::run_with(&options, Some(items))
        .map(|out| out.selected_items)
        .unwrap_or_else(|| Vec::new());

    let command = item.get(0).unwrap().output();
    let mut command = command.split(" ");

    if command.next().unwrap().contains("Window:") {
        let command = format!("[title=\"{}\"] focus", command.next().unwrap());
        ipc.run_command(&command).unwrap();
    } else {
        let command = format!("exec {}", command.next().unwrap());
        ipc.run_command(&command).unwrap();
    }
}

fn path() -> Vec<String> {
    env::var_os("PATH")
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
        .split(":")
        .map(|path| path.to_string())
        .into_iter()
        .sorted()
        .collect::<Vec<String>>()
}

fn running_programs(workspaces: &Vec<Node>) -> Vec<String> {
    let choices: Vec<&Node> = workspaces
        .iter()
        .flat_map(|workspace| &workspace.nodes)
        .collect();

    let choices: Vec<String> = choices
        .into_iter()
        .map(|program| program.name.as_ref().unwrap().to_owned())
        .collect();

    choices
        .iter()
        .map(|program| format!("Window: {}", program))
        .collect::<Vec<String>>()
}

fn launchable_programs(dirs: Vec<String>) -> Vec<String> {
    dirs
        .iter()
        .map(|path| std::fs::read_dir(path))
        .filter(|dir| dir.is_ok())
        .map(|dir| {
            dir.unwrap()
                .map(|dir_entry| dir_entry.unwrap().file_name().to_str().unwrap().to_owned())
                .map(|program| format!("Launch: {}", program))
                .collect::<Vec<String>>()
        })
        .flatten()
        .sorted()
        .dedup()
        .collect::<Vec<String>>()
}
