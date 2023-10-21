use itertools::Itertools;
use simple_logger::SimpleLogger;

const OUTPUT_DIR: &'static str = "build";

use std::{
    error::Error,
    fs::{self, DirEntry, ReadDir},
    iter,
    path::{Path, PathBuf},
};

fn wrap_directory(name: &str, content: &str) -> String {
    include_str!("templates/directory_list.html")
        .replace("{{name}}", name)
        .replace("{{content}}", content)
}

fn wrap_root(ancestors: &[Ancestor], content: &str, name: &str) -> String {
    include_str!("templates/root.html")
        .replace("{{content}}", content)
        .replace("{{breadcrumbs}}", &breadcrumbs_html(ancestors, name))
}

fn breadcrumbs_html(ancestors: &[Ancestor], file_name: &str) -> String {
    let mut previous_path = String::new();
    let mut ancestors = ancestors.into_iter();
    let mut result = Vec::new();
    loop {
        let Some(Ancestor { path, name }) = ancestors.next() else {
            break;
        };
        if previous_path == "/" {
            previous_path = format!("/{path}");
        } else {
            previous_path += &format!("/{path}");
        }
        if ancestors.len() == 0 {
            if file_name != "README.md" {
                result.push(format!(r#"<a href="{previous_path}">{name}</a>"#));
                result.push(format!(r#"<span>{file_name}</span>"#));
            } else {
                result.push(format!(r#"<span>{name}</span>"#));
            }
        } else {
            result.push(format!(r#"<a href="{previous_path}">{name}</a>"#));
        }
    }
    let result = result.join(" / ");
    if result.is_empty() {
        String::new()
    } else {
        result
    }
}

#[derive(Debug)]
enum NodeContent<FileContent, DirectoryContent> {
    Directory(DirectoryContent),
    File(FileContent),
}

#[derive(Debug)]
struct MarkdownNode {
    content: NodeContent<String, (Option<String>, Vec<MarkdownNode>)>,
    file_name: String,
    ancestors: Vec<Ancestor>,
}

#[derive(Debug)]
struct HtmlNode {
    path: PathBuf,
    content: NodeContent<String, (String, Vec<HtmlNode>)>,
}

#[derive(Clone, Debug)]
struct Ancestor {
    name: String,
    path: String,
}

struct FileNode {
    file_name: String,
    path: PathBuf,
    content: NodeContent<String, ReadDir>,
    ancestors: Vec<Ancestor>,
}

impl TryFrom<(Vec<Ancestor>, DirEntry)> for FileNode {
    type Error = String;

    fn try_from((ancestors, entry): (Vec<Ancestor>, DirEntry)) -> Result<Self, Self::Error> {
        Ok(Self {
            ancestors,
            ..entry.try_into()?
        })
    }
}

impl TryFrom<DirEntry> for FileNode {
    type Error = String;

    fn try_from(value: DirEntry) -> Result<Self, Self::Error> {
        let file_name = value
            .file_name()
            .into_string()
            .unwrap_or_else(|file_name| format!("invalid UTF-8 filename: '{file_name:?}'"));

        let path = value.path();

        let metadata = value
            .metadata()
            .map_err(|err| format!("unable to read metadata for '{file_name}': {err}"))?;

        let content = if metadata.is_dir() {
            NodeContent::Directory(
                fs::read_dir(path)
                    .map_err(|err| format!("unable to read directory '{file_name}': {err}"))?,
            )
        } else {
            NodeContent::File(
                fs::read_to_string(path)
                    .map_err(|err| format!("unable to read file '{file_name}': {err}"))?,
            )
        };
        Ok(Self {
            file_name,
            path: value.path(),
            content,
            ancestors: Vec::new(),
        })
    }
}

impl TryFrom<FileNode> for MarkdownNode {
    type Error = String;

    fn try_from(value: FileNode) -> Result<Self, Self::Error> {
        let FileNode {
            path,
            content,
            file_name,
            ancestors,
        } = value;

        match content {
            NodeContent::File(content) => {
                log::info!("parsing file: \"{file_name}\"");
                Ok(MarkdownNode {
                    content: NodeContent::File(content),
                    ancestors,
                    file_name,
                })
            }
            NodeContent::Directory(entries) => {
                log::info!("parsing dir:  \"{file_name}\"");
                let readme_path = path.join("README.md");
                let file_ancestors = Vec::from([
                    ancestors.clone(),
                    vec![Ancestor {
                        name: file_name.clone(),
                        path: if file_name != "root" {
                            file_name.clone()
                        } else {
                            String::new()
                        },
                    }],
                ])
                .concat();

                let children = entries
                    .map(|entry| {
                        entry.map_err(|err| {
                            format!("unable to get child in directory {file_name}: {err}")
                        })
                    })
                    .map(|entry| entry.map(|v| (file_ancestors.clone(), v)))
                    .map(|entry| entry.map(FileNode::try_from)?.map(Self::try_from)?)
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(MarkdownNode {
                    content: NodeContent::Directory((
                        fs::read_to_string(readme_path).ok(),
                        children,
                    )),
                    ancestors,
                    file_name,
                })
            }
        }
    }
}

fn directory_list_html(nodes: &[MarkdownNode]) -> String {
    let content = nodes
        .iter()
        .sorted_by(|a, b| a.file_name.cmp(&b.file_name))
        .map(|node| {
            let class = match node.content {
                NodeContent::Directory(_) => "directory-listing",
                NodeContent::File(_) => "file-listing",
            };
            format!(
                r#"<li class="{class}"><a href="/{}">{}</a></li>"#,
                file_name(&node.ancestors, &node.file_name)
                    .to_str()
                    .unwrap(),
                node.file_name
            )
        })
        .fold(String::new(), |acc, curr| acc + &curr);
    format!("<ul>{content}</ul>",)
}

fn file_name(ancestors: &[Ancestor], name: &str) -> PathBuf {
    let name = match name.strip_suffix("README.md") {
        Some(name) => name.to_string() + "index.html",
        None => name.replace(".md", ".html"),
    };

    let path: PathBuf = ancestors
        .into_iter()
        .chain(iter::once(&Ancestor {
            name: name.clone(),
            path: name.clone(),
        }))
        .filter_map(|item| {
            if item.name == "root" {
                None
            } else {
                Some(item.path.clone())
            }
        })
        .collect();
    path
}

fn file_path(ancestors: &[Ancestor], name: &str) -> PathBuf {
    PathBuf::from("build").join(file_name(ancestors, name))
}

impl From<MarkdownNode> for HtmlNode {
    fn from(node: MarkdownNode) -> Self {
        let content = match node.content {
            NodeContent::File(content) => NodeContent::File(wrap_root(
                &node.ancestors,
                &markdown::to_html(&content),
                &node.file_name,
            )),
            NodeContent::Directory((content, children)) => NodeContent::Directory((
                wrap_root(
                    &node.ancestors,
                    &content
                        .as_ref()
                        .map(|content| markdown::to_html(content))
                        .unwrap_or_else(|| {
                            wrap_directory(&node.file_name, &directory_list_html(&children))
                        }),
                    &node.file_name,
                ),
                children.into_iter().map(Self::from).collect(),
            )),
        };

        HtmlNode {
            path: file_path(&node.ancestors, &node.file_name),
            content,
        }
    }
}

fn write_node_to_dir(node: HtmlNode) -> Result<(), Box<dyn Error>> {
    log::info!("writing to {:?}", node.path);
    match node.content {
        NodeContent::File(content) => fs::write(node.path, content)?,
        NodeContent::Directory((content, children)) => {
            fs::create_dir(&node.path)?;
            fs::write(&node.path.join("index.html"), content)?;
            for node in children {
                write_node_to_dir(node)?;
            }
        }
    }

    Ok(())
}

fn copy_dir_entry<P: AsRef<Path> + Into<PathBuf> + Clone>(
    entry: DirEntry,
    to: P,
) -> Result<(), Box<dyn Error>> {
    let metadata = entry.metadata()?;
    let file_name = entry.file_name();
    let to: PathBuf = to.clone().into();
    let to = to.join(&file_name);

    if metadata.is_dir() {
        fs::create_dir(&to)?;
        fs::read_dir(entry.path())?
            .map(|entry| copy_dir_entry(entry?, to.clone()))
            .collect::<Result<Vec<_>, _>>()?;
    } else {
        log::info!("copying {:?} to {to:?}", entry.path());
        fs::copy(entry.path(), to)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().env().init().unwrap();
    log::info!("cleaning {OUTPUT_DIR}/ directory");
    let _ = fs::remove_dir_all(OUTPUT_DIR)
        .map_err(|_| log::info!("{OUTPUT_DIR}/ directory already empty"));

    let root = FileNode {
        file_name: "root".to_string(),
        path: "articles".into(),
        content: NodeContent::Directory(fs::read_dir("articles")?),
        ancestors: Vec::new(),
    };
    log::info!("");
    log::info!("parsing markdown");
    let root: MarkdownNode = root.try_into()?;
    log::info!("");
    log::info!("compiling to html");
    let root: HtmlNode = root.try_into()?;
    log::info!("");
    write_node_to_dir(root)?;
    log::info!("");
    log::info!("copying contents of public/ to {OUTPUT_DIR}/");
    fs::read_dir("public")?
        .map(|entry| copy_dir_entry(entry?, OUTPUT_DIR))
        .collect::<Result<Vec<_>, _>>()?;
    log::info!("");
    log::info!("done");

    Ok(())
}
