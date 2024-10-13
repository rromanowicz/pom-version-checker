use std::{
    fmt::format,
    fmt::{self, Debug},
    fs,
    process::Command,
};

use regex::Regex;

struct Artifact {
    group_id: Option<String>,
    artifact_id: String,
    version: Option<String>,
}

impl Artifact {
    fn new(artifact_id: &str) -> Self {
        Artifact {
            group_id: None,
            artifact_id: String::from(artifact_id),
            version: None,
        }
    }
}

impl Clone for Artifact {
    fn clone(&self) -> Self {
        Artifact {
            group_id: match &self.group_id {
                Some(v) => Some(String::from(v)),
                None => None,
            },
            artifact_id: String::from(&self.artifact_id),
            version: match &self.version {
                Some(v) => Some(String::from(v)),
                None => None,
            },
        }
    }
}

struct Module {
    artifact: Artifact,
    dependencies: Vec<Artifact>,
    source: String,
}

struct Pom {
    root: Artifact,
    parent: Option<Artifact>,
    modules: Vec<Module>,
    dependencies: Vec<Artifact>,
    source: String,
}

pub fn test(root_dir: &str, skip_group: &str) {
    let parse = Pom::from_file(&root_dir);
    //println!("{:#?}", parse);
    //let parent_1 = match fetch_parent_pom(&parse) {
    //    Some(v) => Some(Pom::from_str(&v)),
    //    None => None,
    //};
    //match parent_1 {
    //    Some(ppom) => {
    //        let parent_2 = match fetch_parent_pom(&ppom) {
    //            Some(v) => Some(Pom::from_str(&v)),
    //            None => None,
    //        };
    //        println!("{:#?}", parent_2);
    //    }
    //    None => todo!(),
    //};

    fetch_parents(&parse)
        .iter()
        .for_each(|p| println!("{:#?}", p));
}

fn fetch_parents(pom: &Pom) -> Vec<Pom> {
    let mut parents = vec![];

    let mut optional = match &pom.parent {
        Some(v) => Some(v.clone()),
        None => None,
    };
    todo!("Change fetch_parent_pom input parameter to 'i");
    while let Some(ref i) = optional {
        let parent = match fetch_parent_pom(optional.clone()) {
            Some(v) => {
                println!("{:?}", &optional);
                let value = Pom::from_str(&v);
                optional = match value.parent {
                    Some(ref v) => Some(v.clone()),
                    None => None,
                };
                parents.push(value);
            }
            None => return parents,
        };
    }

    parents
}

impl Pom {
    fn from_file(path: &str) -> Self {
        let pom = fs::read_to_string(Self::get_root_pom_path(path)).expect("");
        let parent = Self::get_parent(&pom);
        let pom = remove_parent(&pom);
        let root = Self::get_root_artifact(&pom);
        let modules = Self::get_modules(path);
        Pom {
            root,
            parent,
            modules,
            dependencies: get_dependencies(&pom),
            source: pom.replace("\n", "").replace("\t", "").replace(" ", ""),
        }
    }

    fn from_str(pom: &str) -> Self {
        let pom = remove_build(pom);
        let pom = remove_plugins(&pom);

        let parent = Self::get_parent(&pom);
        let pom = remove_parent(&pom);
        let root = Self::get_root_artifact(&pom);
        let modules = vec![];
        Pom {
            root,
            parent,
            modules,
            dependencies: get_dependencies(&pom),
            source: pom.replace("\n", "").replace("\t", "").replace(" ", ""),
        }
    }

    fn get_parent(pom: &str) -> Option<Artifact> {
        let parent_pattern = Regex::new(r"<parent>([\s\S]*?)<\/parent>").unwrap();

        match parent_pattern.captures(pom) {
            Some(v) => {
                let parent = v.get(0).map_or("", |x| x.as_str());
                let artifact = get_artifact(&parent);
                Some(artifact)
            }
            None => None,
        }
    }

    fn get_root_artifact(pom: &str) -> Artifact {
        let pattern = Regex::new(r"(<modelVersion>([\s\S]*?)<\/properties>)").unwrap();
        let find = pattern.find(pom).unwrap();
        get_artifact(find.as_str())
    }

    fn get_root_pom_path(root_dir: &str) -> String {
        let mut path = String::from(root_dir);
        if path.ends_with("/") {
            path += "pom.xml";
        } else {
            path += "/pom.xml";
        }
        path
    }

    fn get_modules(root_dir: &str) -> Vec<Module> {
        let mut path = String::from(root_dir);
        if path.ends_with("/") {
            let mut chars = path.chars();
            chars.next_back();
            path = String::from(chars.as_str());
        }

        let root_pom = format!("{}/pom.xml", &path.split("/").last().unwrap());

        let mut input = Command::new("find");
        input.arg(&path).arg("-name").arg("pom.xml");

        let output = input.output().expect("Error!");

        let mut poms = vec![];
        let value = String::from_utf8(output.stdout).unwrap();

        value.lines().for_each(|l| poms.push(String::from(l)));

        let mut modules = vec![];
        poms.iter().for_each(|pom_path| {
            if !pom_path.ends_with(&root_pom) {
                let pom = fs::read_to_string(pom_path).expect("");
                modules.push(Module::parse(&pom));
            }
        });

        modules
    }
}

impl Module {
    fn parse(pom: &str) -> Self {
        let source = String::from(pom)
            .replace("\n", "")
            .replace("\t", "")
            .replace(" ", "");
        let artifact_pattern = Regex::new(r"<artifactId>(.*)</artifactId>").unwrap();
        let pom = remove_parent(&pom);
        let artifact_id = artifact_pattern
            .captures(&pom)
            .unwrap()
            .get(1)
            .map_or("", |v| v.as_str());

        Module {
            artifact: Artifact::new(artifact_id),
            dependencies: get_dependencies(&pom),
            source,
        }
    }
}

fn get_artifact(input: &str) -> Artifact {
    let group_pattern = Regex::new(r"<groupId>(.*)</groupId>").unwrap();
    let artifact_pattern = Regex::new(r"<artifactId>(.*)</artifactId>").unwrap();
    let version_pattern = Regex::new(r"<version>(.*)</version>").unwrap();
    //todo!();
    let artifact_id = artifact_pattern
        .captures(input)
        .unwrap()
        .get(1)
        .map_or("", |v| v.as_str());
    //let group_id = group_pattern
    //    .captures(input)
    //    .unwrap()
    //    .get(1)
    //    .map_or("", |v| v.as_str());

    let group_id = if let Some(v) = group_pattern.captures(input) {
        Some(v.get(1).map_or("", |v| v.as_str()).to_string())
    } else {
        None
    };

    Artifact {
        group_id,
        artifact_id: String::from(artifact_id),
        version: match version_pattern.captures(input) {
            Some(v) => Some(v.get(1).map_or("", |v| v.as_str()).to_string()),
            None => None, //format!("${{{}.version}}", artifact_id),
        },
    }
}

fn get_dependencies(pom: &str) -> Vec<Artifact> {
    let dependency_pattern = Regex::new(r"<dependency>([\s\S]*?)<\/dependency>").unwrap();
    let mut dependencies = vec![];

    dependency_pattern.captures_iter(&pom).for_each(|f| {
        let artifact = get_artifact(f.get(0).map_or("", |v| v.as_str()));
        dependencies.push(artifact);
    });
    dependencies
}

fn remove_parent(pom: &str) -> String {
    let parent_pattern = Regex::new(r"<parent>([\s\S]*?)<\/parent>").unwrap();
    let result = parent_pattern.replace(pom, "");

    result.to_owned().to_string()
}

fn remove_build(pom: &str) -> String {
    let parent_pattern = Regex::new(r"<build>([\s\S]*?)<\/build>").unwrap();
    let result = parent_pattern.replace(pom, "");

    result.to_owned().to_string()
}

fn remove_plugins(pom: &str) -> String {
    let parent_pattern = Regex::new(r"<plugins>([\s\S]*?)<\/plugins>").unwrap();
    let result = parent_pattern.replace(pom, "");

    result.to_owned().to_string()
}

fn fetch_parent_pom(parent_artifact: Option<Artifact>) -> Option<String> {
    match parent_artifact {
        Some(p) => {
            let mut url_path: String =
                String::from("https://search.maven.org/remotecontent?filepath=");

            let group = p.group_id.clone().unwrap().replace(".", "/");
            url_path.push_str(&group);
            url_path.push_str(format(format_args!("/{}", &p.artifact_id)).as_str());
            url_path.push_str(format(format_args!("/{}", &p.version.as_ref().unwrap())).as_str());
            url_path.push_str(
                format(format_args!(
                    "/{}-{}.pom",
                    &p.artifact_id,
                    &p.version.as_ref().unwrap()
                ))
                .as_str(),
            );
            Some(fetch_from_maven_central(&url_path))
        }
        None => None,
    }
}

fn fetch_from_maven_central(url: &str) -> String {
    let mut input = Command::new("curl");

    input.arg(url);
    let output = input.output().expect("Error!");
    String::from_utf8(output.stdout).unwrap()
}

impl fmt::Debug for Pom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\"pom\": {{\"root\": {:#?}, \"parent\": {:#?}, \"dependencies\": {:#?}, \"modules\": {:#?}}}}}",
            self.root, self.parent, self.dependencies, self.modules
        )
    }
}

impl fmt::Debug for Artifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\"groupId\": {:#?},\"artifactId\": {:#?}, \"version\": {:#?}}}",
            self.group_id, self.artifact_id, self.version
        )
    }
}

impl fmt::Debug for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\"artifactId\": {:#?}, \"dependencies\": [{:#?}]}}",
            self.artifact, self.dependencies
        )
    }
}
