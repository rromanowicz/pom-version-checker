use regex::Regex;
use std::{env, fmt::format, fs, process::Command};

mod pom;

fn main() {
    let args: Vec<String> = env::args().collect();
    let skip_group = &args[1];
    let dir = &args[2];

    let pom_list = get_pom_list(dir);

    //pom_list
    //    .iter()
    //    .for_each(|pom| check_pom_versions(pom, skip_group));

    pom::test(dir, skip_group);
}

fn get_pom_list(dir: &str) -> Vec<String> {
    let mut input = Command::new("find");
    input.arg(dir).arg("-name").arg("pom.xml");

    let output = input.output().expect("Error!");

    let mut poms = vec![];
    let value = String::from_utf8(output.stdout).unwrap();

    value.lines().for_each(|l| poms.push(String::from(l)));

    poms.sort();

    poms
}

fn testing(pom: &str, skip_group: &str) {
    println!("{}", pom);

    let pom = fs::read_to_string(pom).expect("");

    let dependency_pattern = Regex::new(
        r"<dependency>(.*|\n.*)<groupId>(.*)</groupId>(.*|\n.*)<artifactId>(.*)</artifactId>((.*|\n.*)<version>(.*)</version>)*",
    ).unwrap();

    let parent_dep = get_parent(&pom, skip_group);

    let dependencies_pom = get_spring_boot_dependencies(&parent_dep);
    let parent_dep = match parent_dep {
        Some(v) => v,
        None => Dependency::new(),
    };

    let mut dependencies = get_dependencies(&pom, skip_group);
    dependencies
        .iter()
        .for_each(|v| println!("\t{}", v.to_string()));
}

fn check_pom_versions(pom: &str, skip_group: &str) {
    println!("{}", pom);

    let pom = fs::read_to_string(pom).expect("");

    let dependency_pattern = Regex::new(
        r"<dependency>(.*|\n.*)<groupId>(.*)</groupId>(.*|\n.*)<artifactId>(.*)</artifactId>((.*|\n.*)<version>(.*)</version>)*",
    ).unwrap();

    let parent_dep = get_parent(&pom, skip_group);

    let dependencies_pom = get_spring_boot_dependencies(&parent_dep);
    let parent_dep = match parent_dep {
        Some(v) => v,
        None => Dependency::new(),
    };

    let mut dependencies = vec![];

    dependency_pattern.captures_iter(&pom).for_each(|f| {
        let group_id = f.get(2).map_or("", |v| v.as_str());
        let artifact_id = f.get(4).map_or("", |v| v.as_str());
        let version_opt = f.get(7).map_or(None, |v| Some(v.as_str().to_string()));
        let mut version = match version_opt {
            Some(v) => v,
            None => format!("${{{}.version}}", artifact_id),
        };
        let mut parent = None;
        if version.starts_with("$") {
            match get_property(&pom, &version) {
                Some(local_value) => version = local_value,
                None => match &dependencies_pom {
                    Some(value) => match get_property(&value, &version) {
                        Some(parent_value) => {
                            parent = Some(String::from(&parent_dep.to_string()));
                            version = parent_value
                        }
                        None => {
                            version = if artifact_id.starts_with("spring-boot-") {
                                parent = Some(String::from(&parent_dep.artifact_id));
                                String::from(&parent_dep.version)
                            } else {
                                String::from("#####")
                            }
                        }
                    },
                    None => (),
                },
            }
        }
        if !group_id.starts_with(skip_group) {
            dependencies.push(Dependency {
                group_id: group_id.to_string(),
                artifact_id: artifact_id.to_string(),
                version,
                parent,
            });
        }
    });

    let mut standalone = vec![];
    let mut from_parent = vec![];
    dependencies.iter().for_each(|dep| match dep.parent {
        Some(_) => from_parent.push(dep),
        None => standalone.push(dep),
    });

    if dependencies.len() > 0 {
        if standalone.len() > 0 {
            println!("Standalone:");
            standalone
                .iter()
                .for_each(|v| println!(" -{} -> {}", v.to_string(), get_latest_version(&v)));
        }
        println!("");
        if from_parent.len() > 0 {
            println!("Parent: [{}]", &parent_dep.artifact_id);
            from_parent
                .iter()
                .for_each(|v| println!(" -{} -> {}", v.to_string(), get_latest_version(&v)));
        }
    }
}

fn get_parent(pom: &str, skip_group: &str) -> Option<Dependency> {
    let parent_pattern = Regex::new(r"<parent>([\s\S]*?)<\/parent>").unwrap();

    match parent_pattern.captures(pom) {
        Some(v) => {
            let parent = v.get(0).map_or("", |x| x.as_str());
            let artifact = get_artifact(&parent);
            if artifact.group_id.starts_with(skip_group) {
                None
            } else {
                Some(artifact)
            }
        }
        None => None,
    }
}

fn get_dependencies(pom: &str, skip_group: &str) -> Vec<Dependency> {
    let dependency_pattern = Regex::new(r"<dependency>([\s\S]*?)<\/dependency>").unwrap();
    let mut dependencies = vec![];

    dependency_pattern.captures_iter(&pom).for_each(|f| {
        let artifact = get_artifact(f.get(0).map_or("", |v| v.as_str()));
        if !artifact.group_id.starts_with(skip_group) {
            dependencies.push(artifact);
        }
    });
    dependencies
}

fn get_artifact(input: &str) -> Dependency {
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
        .unwrap()
        .get(1)
        .map_or("", |v| v.as_str());
    Dependency {
        group_id: String::from(group_id),
        artifact_id: String::from(artifact_id),
        version: match version_pattern.captures(input) {
            Some(v) => v.get(1).map_or("", |v| v.as_str()).to_string(),
            None => format!("${{{}.version}}", artifact_id),
        },
        parent: None,
    }
}

fn get_spring_boot_dependencies(parent: &Option<Dependency>) -> Option<String> {
    match parent {
        Some(p) => {
            let dependencies_art: String = String::from("spring-boot-dependencies");
            let mut url_path: String =
                String::from("https://search.maven.org/remotecontent?filepath=");

            let group = p.group_id.replace(".", "/");
            url_path.push_str(&group);
            url_path.push_str(format(format_args!("/{}", &dependencies_art)).as_str());
            url_path.push_str(format(format_args!("/{}", &p.version)).as_str());
            url_path.push_str(
                format(format_args!("/{}-{}.pom", &dependencies_art, &p.version)).as_str(),
            );
            Some(get_deps(&url_path))
        }
        None => None,
    }
}

fn get_deps(url: &str) -> String {
    let mut input = Command::new("curl");

    input.arg(url);
    let output = input.output().expect("Error!");
    String::from_utf8(output.stdout).unwrap()
}

fn get_property(pom: &str, artifact: &str) -> Option<String> {
    let art = artifact.replace("${", "").replace("}", "");
    let prop_ptrn = format!(r"<{}>(.*)</{}>", art, art);
    let reg = Regex::new(&prop_ptrn).unwrap();

    match reg.captures(pom) {
        Some(res) => res.get(1).map_or(None, |v| Some(v.as_str().to_string())),
        None => None,
    }
}

fn get_latest_version(dep: &Dependency) -> String {
    String::new()
    //let mut input = Command::new("sh");
    //
    //input
    //    .arg("mvn_latest_version.sh")
    //    .arg("-g")
    //    .arg(&dep.group_id)
    //    .arg("-a")
    //    .arg(&dep.artifact_id);
    //let output = input.output().expect("Error!");
    //let mut version = String::from_utf8(output.stdout).unwrap();
    //version.retain(|c| !c.is_whitespace());
    //version
}

#[allow(dead_code)]
#[derive(Debug)]
struct Dependency {
    group_id: String,
    artifact_id: String,
    version: String,
    parent: Option<String>,
}

impl Dependency {
    fn new() -> Self {
        Dependency {
            group_id: String::new(),
            artifact_id: String::new(),
            version: String::new(),
            parent: None,
        }
    }
    fn to_string(&self) -> String {
        format!("{}:{}:{}", self.group_id, self.artifact_id, self.version)
    }
}
