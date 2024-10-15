use std::{
    fmt::{self, format},
    fs,
    process::Command,
};

use regex::Regex;

struct Artifact {
    group_id: Option<String>,
    artifact_id: String,
    version: Option<String>,
    latest_version: Option<String>,
}

impl Artifact {
    fn new(artifact_id: &str) -> Self {
        Artifact {
            group_id: None,
            artifact_id: String::from(artifact_id),
            version: None,
            latest_version: None,
        }
    }
}

impl Clone for Artifact {
    fn clone(&self) -> Self {
        Artifact {
            group_id: self.group_id.as_ref().map(String::from),
            artifact_id: String::from(&self.artifact_id),
            version: self.version.as_ref().map(String::from),
            latest_version: None,
        }
    }
}

#[allow(dead_code)]
struct Module {
    artifact: Artifact,
    dependencies: Vec<Artifact>,
    source: String,
}

impl Clone for Module {
    fn clone(&self) -> Self {
        Module {
            artifact: self.artifact.clone(),
            dependencies: self.dependencies.clone(),
            source: self.source.clone(),
        }
    }
}

#[allow(dead_code)]
pub struct Pom {
    root: Artifact,
    parent: Option<Artifact>,
    modules: Vec<Module>,
    dependencies: Vec<Artifact>,
    source: String,
    skip_group: Option<String>,
}
impl Clone for Pom {
    fn clone(&self) -> Self {
        Pom {
            root: self.root.clone(),
            parent: self.parent.clone(),
            modules: self.modules.clone(),
            dependencies: self.dependencies.clone(),
            source: self.source.clone(),
            skip_group: self.skip_group.clone(),
        }
    }
}

fn get_version_from_parents(prop: &Artifact, parents: &[&Pom]) -> Option<String> {
    let mut version: Option<String> = None;
    for parent in parents.iter() {
        let artifact_version = match &prop.version {
            Some(v) => String::from(v),
            None => String::new(),
        };
        let properties_version = get_property(&artifact_version, &parent.source);
        match properties_version {
            Some(v) => {
                if !&v.starts_with("$") {
                    version = Some(v);
                    break;
                }
                let dependencies_version = get_version_from_parent_dependencies(prop, parent);
                if dependencies_version.is_some() {
                    version = dependencies_version;
                    break;
                }
            }
            None => {
                let dependencies_version = get_version_from_parent_dependencies(prop, parent);
                if dependencies_version.is_some() {
                    version = dependencies_version;
                    break;
                }
            }
        }
    }
    version
}

fn get_version_from_parent_dependencies(prop: &Artifact, parent: &Pom) -> Option<String> {
    let mut version: Option<String> = None;
    for dep in parent.dependencies.clone() {
        if dep.artifact_id.eq(&prop.artifact_id) {
            version = dep.version;
        }
    }
    version
}

impl Pom {
    pub fn from_file(path: &str, skip_group: &str) -> Self {
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
            skip_group: Some(String::from(skip_group)),
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
            skip_group: None,
        }
    }

    fn get_parent(pom: &str) -> Option<Artifact> {
        let parent_pattern = Regex::new(r"<parent>([\s\S]*?)<\/parent>").unwrap();

        match parent_pattern.captures(pom) {
            Some(v) => {
                let parent = v.get(0).map_or("", |x| x.as_str());
                let artifact = get_artifact(parent);
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

    pub fn fetch_parents(&self) -> Vec<Pom> {
        let mut parents = vec![];

        let mut optional = self.parent.clone();
        while let Some(ref _i) = optional {
            match fetch_parent_pom(optional.clone()) {
                Some(v) => {
                    let value = Pom::from_str(&v);
                    optional = value.parent.clone();
                    parents.push(value);
                }
                None => return parents,
            };
        }

        parents
    }

    pub fn fetch_latest_versions(&mut self) {
        println!("{:?}", self.root.artifact_id);
        self.dependencies.iter_mut().for_each(|dep| {
            if let Some(group) = &dep.group_id {
                if let Some(sg) = &self.skip_group {
                    if !group.starts_with(sg) {
                        dep.latest_version = Some(get_latest_version(dep));
                        if dep.version != dep.latest_version {
                            println!("\t{:?}", &dep);
                        }
                    }
                }
            }
        });

        self.modules.iter_mut().for_each(|module| {
            println!("{:?}", module.artifact.artifact_id);
            module.dependencies.iter_mut().for_each(|dep| {
                if let Some(group) = &dep.group_id {
                    if let Some(sg) = &self.skip_group {
                        if !group.starts_with(sg) {
                            dep.latest_version = Some(get_latest_version(dep));
                            if dep.version != dep.latest_version {
                                println!("\t{:?}", &dep);
                            }
                        }
                    }
                }
            });
        });
    }

    pub fn fill_missing_properties(&mut self, parents: &[Pom]) -> &mut Self {
        let mut zxc = vec![];
        parents.iter().for_each(|p| zxc.push(p));

        self.dependencies.iter_mut().for_each(|dep| {
            dep.version = {
                match &dep.version {
                    Some(v) => {
                        if v.starts_with("$") {
                            get_version_from_parents(dep, &zxc)
                        } else {
                            Some(v.to_string())
                        }
                    }
                    None => {
                        let art_id = format!("${{{}.version}}", &dep.artifact_id);
                        dep.version = Some(art_id);
                        get_version_from_parents(dep, &zxc)
                    }
                }
            }
        });

        let mut p2 = vec![];
        parents.iter().for_each(|p| p2.push(p));
        let binding = self.clone();
        p2.push(&binding);
        self.modules.iter_mut().for_each(|module| {
            module.dependencies.iter_mut().for_each(|dep| {
                dep.version = {
                    match &dep.version {
                        Some(v) => {
                            if v.starts_with("$") {
                                get_version_from_parents(dep, &p2)
                            } else {
                                Some(v.to_string())
                            }
                        }
                        None => {
                            let art_id = format!("${{{}.version}}", &dep.artifact_id);
                            dep.version = Some(art_id);
                            get_version_from_parents(dep, &p2)
                        }
                    }
                }
            })
        });

        self
    }
}

impl Module {
    fn parse(pom: &str) -> Self {
        let source = String::from(pom)
            .replace("\n", "")
            .replace("\t", "")
            .replace(" ", "");
        let artifact_pattern = Regex::new(r"<artifactId>(.*)</artifactId>").unwrap();
        let pom = remove_parent(pom);
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

    let artifact_id = artifact_pattern
        .captures(input)
        .unwrap()
        .get(1)
        .map_or("", |v| v.as_str());

    let group_id = group_pattern
        .captures(input)
        .map(|v| v.get(1).map_or("", |v| v.as_str()).to_string());

    Artifact {
        group_id,
        artifact_id: String::from(artifact_id),
        version: version_pattern
            .captures(input)
            .map(|v| v.get(1).map_or("", |v| v.as_str()).to_string()),
        latest_version: None,
    }
}

fn get_dependencies(pom: &str) -> Vec<Artifact> {
    let dependency_pattern = Regex::new(r"<dependency>([\s\S]*?)<\/dependency>").unwrap();
    let mut dependencies = vec![];

    dependency_pattern.captures_iter(pom).for_each(|f| {
        let mut artifact = get_artifact(f.get(0).map_or("", |v| v.as_str()));
        if let Some(ref v) = artifact.version {
            if v.starts_with("$") {
                artifact.version = get_property(v, pom);
            }
        }
        dependencies.push(artifact);
    });
    dependencies
}

fn get_property(prop: &str, pom: &str) -> Option<String> {
    let properties_pattern = Regex::new(r"<properties>([\s\S]*?)<\/properties>").unwrap();
    let properties = match properties_pattern.captures(pom) {
        Some(props) => props.get(0).map_or("", |v| v.as_str()),
        None => "",
    };

    let version_tag = prop.replace("${", "").replace("}", "");
    let property_pattern_str = format!(r"<{}>(.*)</{}>", version_tag, version_tag);
    let property_pattern = Regex::new(&property_pattern_str).unwrap();

    match property_pattern.captures(properties) {
        Some(res) => res.get(1).map(|v| v.as_str().to_string()),
        None => Some(String::from(prop)),
    }
}

fn remove_parent(pom: &str) -> String {
    let parent_pattern = Regex::new(r"<parent>([\s\S]*?)<\/parent>").unwrap();
    let result = parent_pattern.replace(pom, "");

    result.clone().to_string()
}

fn remove_build(pom: &str) -> String {
    let parent_pattern = Regex::new(r"<build>([\s\S]*?)<\/build>").unwrap();
    let result = parent_pattern.replace(pom, "");

    result.clone().to_string()
}

fn remove_plugins(pom: &str) -> String {
    let parent_pattern = Regex::new(r"<plugins>([\s\S]*?)<\/plugins>").unwrap();
    let result = parent_pattern.replace(pom, "");

    result.clone().to_string()
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

fn get_latest_version(art: &Artifact) -> String {
    let mut input = Command::new("sh");

    input
        .arg("mvn_latest_version.sh")
        .arg("-g")
        .arg(match &art.group_id {
            Some(g) => g,
            None => "",
        })
        .arg("-a")
        .arg(&art.artifact_id);
    let output = input.output().expect("Error!");
    let mut version = String::from_utf8(output.stdout).unwrap();
    version.retain(|c| !c.is_whitespace());
    version
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
            "{{\"groupId\": {:#?},\"artifactId\": {:#?}, \"version\": {:#?}, \"latestVersion\": {:#?}}}",
            self.group_id.clone().unwrap_or_default(), self.artifact_id, self.version.clone().unwrap_or_default(), self.latest_version.clone().unwrap_or_default()
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
