use itertools::Itertools;
use skim::{prelude::*, AnsiString};
use swayipc::{reply::Node, Connection};

fn main() {
    // Makes a channel to send items
    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

    let mut ipc = Connection::new().unwrap();
    if let Ok(tree) = ipc.get_tree() {
        let nodes = &tree.nodes.get(1).unwrap();

        get_running_programs_from(&nodes.nodes)
            .into_iter()
            .for_each(|item| {
                let _ = tx_item.send(Arc::new(item));
            });
    } else {
        print!("application switcher does not work on i3 sorry\n");
    }

    get_launchable_programs(&path())
        .into_iter()
        .for_each(|item| {
            let _ = tx_item.send(Arc::new(item));
        });

    //We drop the tranmssion pipe, so skim does know too stop listing after more items
    drop(tx_item);

    let options = SkimOptionsBuilder::default()
        .multi(false)
        // .preview(Some(""))
        .build()
        .unwrap();

    // `run_with` would read and show items from the stream
    let item = Skim::run_with(&options, Some(rx_item))
        .map(|out| out.selected_items)
        .unwrap_or_else(|| Vec::new());

    let item = item.get(0).expect("You did not select anyitems").to_owned();

    let command: &SwitchType = (*item)
        .as_any()
        .downcast_ref()
        .expect("Something wrong with downcast ");

    command.action(&mut ipc);
}

fn get_running_programs_from(workspaces: &Vec<Node>) -> Vec<SwitchType> {
    let choices: Vec<&Node> = workspaces
        .iter()
        .flat_map(|workspace| &workspace.nodes)
        .collect();

    let choices: Vec<String> = choices
        .into_iter()
        .map(|program| program.name.as_ref())
        .flatten()
        .map(|program| program.to_owned())
        .collect();

    choices
        .iter()
        .map(|program| SwitchType::Focus(program.to_owned()))
        .collect::<Vec<SwitchType>>()
}

fn path() -> Vec<String> {
    std::env::var_os("PATH")
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
        .split(":")
        .map(|path| path.to_string())
        .into_iter()
        .sorted()
        .dedup()
        .collect::<Vec<String>>()
}

fn get_launchable_programs(paths: &Vec<String>) -> Vec<SwitchType> {
    paths
        .iter()
        .map(|path| std::fs::read_dir(path))
        .filter(|dir| dir.is_ok())
        .map(|dir| {
            dir.unwrap()
                .map(|dir_entry| dir_entry.unwrap().file_name().to_str().unwrap().to_owned())
                .map(|program| SwitchType::Launch(program))
                .collect::<Vec<SwitchType>>()
        })
        .flatten()
        .sorted()
        .dedup()
        .collect::<Vec<SwitchType>>()
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum SwitchType {
    Launch(String),
    Focus(String),
}

impl SwitchType {
    fn action(&self, ipc: &mut Connection) {
        match self {
            SwitchType::Launch(name) => ipc.run_command(&format!("exec {}", name)).unwrap(),
            SwitchType::Focus(name) => {
                let temp: String = name
                    .text()
                    .chars()
                    .map(|chars| match chars {
                        '(' => "\\(".to_string(),
                        ')' => "\\)".to_string(),
                        ':' => "\\:".to_string(),
                        _ => chars.to_string(),
                    })
                    .collect();
                ipc.run_command(&format!("[title=\"{}\"] focus", temp))
                    .unwrap()
            }
        };
    }
}

impl SkimItem for SwitchType {
    fn display(&self) -> Cow<AnsiString> {
        match &self {
            SwitchType::Launch(name) => {
                Cow::Owned(AnsiString::parse(&format!("\x1b[32m{}\x1b[m", name)))
            }
            SwitchType::Focus(name) => {
                Cow::Owned(AnsiString::parse(&format!("\x1b[4m\x1b[34m{}\x1b[m", name)))
            }
        }
    }
    fn text(&self) -> Cow<str> {
        match &self {
            SwitchType::Launch(name) => Cow::Borrowed(name),
            SwitchType::Focus(name) => Cow::Borrowed(name),
        }
    }
    fn preview(&self) -> ItemPreview {
        match &self {
            SwitchType::Launch(name) => {
                ItemPreview::AnsiText(format!("\x1b[4m\x1b[32mLaunch:\x1b[m\n{}", name))
            }
            SwitchType::Focus(name) => {
                ItemPreview::AnsiText(format!("\x1b[4m\x1b[34mFocus:\x1b[m\n{}", name))
            }
        }
    }
}
