use env_logger::{init_from_env, Env};
use itertools::Itertools;
use log::{error, info};
use skim::{prelude::*, AnsiString};
use swayipc::Connection;

fn main() {
    init_from_env(Env::default().filter_or("SKIM_LOG", "info"));

    let mut connection = Connection::new().expect("Does sway run?");

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

    get_running_programs_from_sway(&mut connection, &tx_item);
    get_launchable_programs(&get_paths_to_bin(), &tx_item);
    //We drop the tranmssion pipe, so skim does know too stop listing after more items
    drop(tx_item);

    if let Ok(options) = SkimOptionsBuilder::default().multi(false).build() {
        // `run_with` would read and show items from the stream
        let item = Skim::run_with(&options, Some(rx_item))
            .map(|out| out.selected_items)
            .unwrap_or_else(|| Vec::new());

        if let Some(item) = item.get(0) {
            if let Some(command) = (**item).as_any().downcast_ref::<SwitchType>() {
                command.action(&mut connection);
            } else {
                error!("Can't cast to SwitchType");
            }
        } else {
            info!("Please select a item");
        }
    }
}

fn get_running_programs_from_sway(connection: &mut Connection, sender: &SkimItemSender) {
    let tree = connection.get_tree().expect("No tree en sway");
    tree.nodes
        .iter()
        .flat_map(|workspace| &workspace.nodes)
        .flat_map(|workspace_node| &workspace_node.nodes)
        // .flat_map(|maybe_program| maybe_program.name.as_ref())
        .map(|program| SwitchType::Focus(program.app_id.clone(), program.name.clone()))
        .for_each(|switcher| {
            let _ = sender.send(Arc::new(switcher));
        });
}

fn get_paths_to_bin() -> Vec<String> {
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

fn get_launchable_programs(paths: &Vec<String>, sender: &SkimItemSender) {
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
        .for_each(|item| {
            let _ = sender.send(Arc::new(item));
        });
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum SwitchType {
    Launch(String),
    Focus(Option<String>, Option<String>),
}

impl SwitchType {
    fn action(&self, connection: &mut Connection) {
        match self {
            SwitchType::Launch(name) => {
                connection.run_command(&format!("exec {}", name)).unwrap();
            }
            SwitchType::Focus(_, Some(title)) => {
                let temp: String = title
                    .text()
                    .chars()
                    .map(|chars| match chars {
                        '(' => "\\(".to_string(),
                        ')' => "\\)".to_string(),
                        ':' => "\\:".to_string(),
                        _ => chars.to_string(),
                    })
                    .collect();
                connection
                    .run_command(&format!("[title=\"{}\"] focus", temp))
                    .unwrap();
            }
            SwitchType::Focus(_, _) => {}
        };
    }
}

impl SkimItem for SwitchType {
    fn display(&self, _context: DisplayContext) -> AnsiString {
        match &self {
            SwitchType::Launch(name) => AnsiString::parse(&format!("\x1b[32m{}\x1b[m", name)),
            SwitchType::Focus(Some(program_name), Some(name)) => {
                AnsiString::parse(&format!("\x1b[4m\x1b[34m{} - {}\x1b[m", program_name, name))
            }
            SwitchType::Focus(None, Some(name)) => {
                AnsiString::parse(&format!("\x1b[4m\x1b[34m{}\x1b[m", name))
            }
            SwitchType::Focus(_, _) => AnsiString::parse(" NONE "),
        }
    }
    fn text(&self) -> Cow<str> {
        match &self {
            SwitchType::Launch(name) => Cow::Borrowed(name),
            SwitchType::Focus(_, Some(title)) => Cow::Borrowed(title),
            SwitchType::Focus(_, _) => Cow::Borrowed(" NONE "),
        }
    }
    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        match &self {
            SwitchType::Launch(name) => {
                ItemPreview::AnsiText(format!("\x1b[4m\x1b[32mLaunch:\x1b[m\n{}", name))
            }
            SwitchType::Focus(_, Some(title)) => {
                ItemPreview::AnsiText(format!("\x1b[4m\x1b[34mFocus:\x1b[m\n{}", title))
            }
            SwitchType::Focus(_, _) => ItemPreview::AnsiText(" NONE ".to_string()),
        }
    }
}
