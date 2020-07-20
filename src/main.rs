extern crate skim;
use i3ipc::reply::Node;
use i3ipc::I3Connection;
use skim::prelude::*;
use std::env;
use std::io::Cursor;

fn main() {
    let mut ipc = I3Connection::connect().expect("failed to connect");
    let tree = ipc.get_tree().unwrap();
    let i3node: &Node = tree.nodes.get(1).unwrap();
    // let deeper: &Node = i3node.nodes.get(2).unwrap();
    // let programs = i3node.nodes.iter().map(|workspaces | workspaces.nodes.iter().map(|nodes| nodes.window_properties.map(|win| win.get(&i3ipc::reply::WindowProperty::WindowRole))).collect());
    let workspaces: &Vec<Node> = &i3node.nodes;
    let programs: Vec<&Node> = workspaces
        .iter()
        .flat_map(|workspace| &workspace.nodes)
        .collect();

    let programs: Vec<String> = programs
        .into_iter()
        .map(|program| program.name.as_ref().unwrap().to_owned())
        .collect();

    let mut programs: Vec<String> = programs
        .iter()
        .map(|program| format!("Window: {}", program))
        .collect();

    list_programs()
        .into_iter()
        .map(|program| format!("Launch: {}", program))
        .for_each(|prog| programs.push(prog));

    programs.iter_mut().for_each(|program| program.push('\n'));
    let input: String = programs.into_iter().map(|program| program).collect();

    // `SkimItemReader` is a helper to turn any `BufRead` into a stream of `SkimItem`
    // `SkimItem` was implemented for `AsRef<str>` by default
    let item_reader = SkimItemReader::default();
    let items = item_reader.of_bufread(Cursor::new(input));

    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .multi(false)
        .build()
        .unwrap();
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

fn list_programs() -> Vec<String> {
    let paths = env::var_os("PATH").unwrap();
    let paths: String = paths.to_str().unwrap().into();
    let mut paths: Vec<String> = paths.split(":").map(|path| path.to_string()).collect();
    paths.sort();
    paths.dedup();
    let mut files: Vec<String> = vec![];
    for path in paths.iter() {
        match read_dir(path) {
            Some(mut test) => files.append(&mut test),
            None => {}
        }
    }
    files.sort();
    files.dedup();
    files
}

fn read_dir(path: &str) -> Option<Vec<String>> {
    match std::fs::read_dir(path) {
        Ok(path) => Some(
            path.map(|file| file.unwrap().file_name().to_str().unwrap().to_owned())
                .collect(),
        ),
        Err(_) => None,
    }
}
